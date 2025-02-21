use darling::{error::Accumulator, util::SpannedValue, FromMeta, Result};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, FnArg, Generics, Ident, Pat, Receiver, ReturnType,
    Signature, Token, Type,
};
use tap::{Pipe, Tap};

use crate::util::{
    tpl::{return_type, Arity, Call},
    Feature, FeatureName, NonFatalErrors,
};

use super::{property_key, self_arg, Argument, Constructor, Function, MaybeAsync, This, TypeCast};

pub enum Callable {
    Func(Function),
    Ctor(Constructor),
}

struct Callee {
    cast: TypeCast,
    this: This,
    ctor: bool,
    name: SpannedValue<String>,
    err_ctx: String,
    self_arg: Option<Receiver>,
    fn_color: MaybeAsync,
}

impl Callee {
    fn from_func(
        Function { name, this, cast }: Function,
        sig: &Signature,
        errors: &mut Accumulator,
    ) -> Self {
        errors.handle(cast.option_check::<Function>(&sig.output));
        let name = property_key(&sig.ident, &name);
        Self {
            cast,
            this,
            ctor: false,
            err_ctx: format!("failed to call function {:?}", name.as_str()),
            name: name.into_owned(),
            self_arg: errors
                .handle(self_arg::<Function>(&sig.inputs, sig.span()))
                .cloned(),
            fn_color: MaybeAsync::some::<Function>(sig).non_fatal(errors),
        }
    }

    fn from_ctor(
        Constructor { class }: Constructor,
        sig: &Signature,
        errors: &mut Accumulator,
    ) -> Self {
        let cast = TypeCast::V8;
        errors.handle(cast.option_check::<Constructor>(&sig.output));
        let name = match (class, &sig.output) {
            (Some(class), _) => class,
            (None, ReturnType::Type(_, ty)) => {
                let ident = match &**ty {
                    Type::Path(ty) => ty.path.segments.last().map(|s| &s.ident),
                    _ => None,
                };
                if let Some(ident) = ident {
                    SpannedValue::new(ident.to_string(), ident.span())
                } else {
                    "cannot infer class name from return type\nspecify `#[js(new(class(...)))]` instead"
                        .pipe(Constructor::error)
                        .with_span(ty)
                        .pipe(|e| errors.push(e));
                    property_key(&sig.ident, &None).into_owned()
                }
            }
            (None, ReturnType::Default) => {
                "cannot infer class name\nspecify a return type, or use `#[js(new(class(...)))]`"
                    .pipe(Constructor::error)
                    .with_span(&sig.ident)
                    .pipe(|e| errors.push(e));
                property_key(&sig.ident, &None).into_owned()
            }
        };
        Self {
            cast,
            this: This::Self_,
            ctor: true,
            err_ctx: format!("failed to construct {:?}", name.as_str()),
            name,
            self_arg: errors
                .handle(self_arg::<Constructor>(&sig.inputs, sig.span()))
                .cloned(),
            fn_color: MaybeAsync::Sync.only::<Constructor>(sig).non_fatal(errors),
        }
    }
}

pub fn impl_function(call: Callable, sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Accumulator::default();

    let Callee {
        cast,
        this,
        ctor,
        name,
        err_ctx,
        self_arg,
        fn_color,
    } = match call {
        Callable::Func(func) => Callee::from_func(func, &sig, &mut errors),
        Callable::Ctor(ctor) => Callee::from_ctor(ctor, &sig, &mut errors),
    };

    let Signature {
        ident,
        generics,
        inputs,
        output,
        ..
    } = sig;

    let Generics {
        params,
        where_clause,
        ..
    } = generics;

    let return_ty = return_type(&output);

    let mut arguments = Vec::<(Ident, Argument)>::new();

    let inputs = inputs
        .into_iter()
        .skip(1)
        .map(|arg| {
            let FnArg::Typed(arg) = arg else { return arg };

            let (options, arg) = {
                let mut arg = arg;

                let (options, attrs) =
                    Feature::<JsArgument>::collect(arg.attrs).non_fatal(&mut errors);

                arg.attrs = attrs;

                if options.len() > 1 {
                    JsArgument::error("more than one specified")
                        .with_span(&arg)
                        .pipe(|e| errors.push(e))
                }

                let options = match options.into_iter().next() {
                    Some(Feature(JsArgument::Arg(options))) => options,
                    None => Default::default(),
                };

                (options, arg)
            };

            let Pat::Ident(name) = &*arg.pat else {
                JsArgument::error("patterns are not supported here")
                    .with_span(&arg.pat)
                    .pipe(|e| errors.push(e));
                return FnArg::Typed(arg);
            };

            if let Some((sub, _)) = &name.subpat {
                JsArgument::error("subpatterns are not supported here")
                    .with_span(sub)
                    .pipe(|e| errors.push(e));
            }

            arguments.push((name.ident.clone(), options));

            FnArg::Typed(arg)
        })
        .collect::<Punctuated<FnArg, Token![,]>>();

    let arguments = arguments;

    let (casts, arity) = scan_args(this, &arguments, quote! { _rt }, quote! { self }, &err_ctx);

    let fn_call = Call { name, ctor, arity }.render();

    let await_retval = match fn_color {
        MaybeAsync::Async(_) => quote! {
            _rt.resolve(retval).await?
        },
        MaybeAsync::Sync => quote! {
            retval
        },
    };

    let into_retval = match &output {
        ReturnType::Default => {
            quote! {
                let _ = retval;
                Ok(())
            }
        }
        ReturnType::Type(..) => {
            let from_retval = cast.from_v8_local("retval", "scope");
            let into_retval = cast.into_return_value("retval");
            quote! {{
                let scope = &mut _rt.handle_scope();
                let retval = v8::Local::new(scope, retval);
                let retval = #from_retval
                    .context("failed to convert returned value")
                    .context(#err_ctx)?;
                #into_retval
            }}
        }
    };

    let inputs = inputs.tap_mut(|p| {
        if !p.empty_or_trailing() {
            p.push_punct(Token![,](Span::call_site()));
        }
    });

    let impl_ = quote! {
        #fn_color fn #ident <#params> (
            #self_arg,
            #inputs
            _rt: &mut JsRuntime,
        ) -> Result<#return_ty>
        #where_clause
        {
            let args = #casts;

            let retval = {
                #fn_call
                let scope = &mut _rt.handle_scope();
                let this = AsRef::<v8::Global<_>>::as_ref(self);
                let this = v8::Local::new(scope, this);
                call(scope, this, args)
                    .context(#err_ctx)?
            };

            let retval = #await_retval;

            #into_retval
        }
    };

    errors.finish()?;

    Ok(vec![impl_])
}

impl From<Function> for Callable {
    fn from(value: Function) -> Self {
        Self::Func(value)
    }
}

impl From<Constructor> for Callable {
    fn from(value: Constructor) -> Self {
        Self::Ctor(value)
    }
}

#[derive(Debug, Clone, FromMeta)]
enum JsArgument {
    Arg(Argument),
}

impl FeatureName for JsArgument {
    const PREFIX: &str = "js";

    fn unit() -> Result<Self> {
        JsArgument::from_word()
    }
}

fn scan_args<T>(
    this: This,
    args: &[(Ident, Argument)],
    rt: T,
    self_: T,
    ctx: &str,
) -> (TokenStream, Arity)
where
    T: ToTokens,
{
    let scope = quote! {
        let __scope = &mut #rt.handle_scope();
    };

    let this = match this {
        This::Self_ => quote! {{
            let this = AsRef::<v8::Global<_>>::as_ref(#self_);
            let this = v8::Local::new(__scope, this);
            let this = this.try_cast::<v8::Value>()
                .context("failed to serialize `this`")
                .context(#ctx)?;
            v8::Global::new(__scope, this)
        }},
        This::Undefined => quote! {{
            let this = v8::undefined(__scope);
            let this = this.cast::<v8::Value>();
            v8::Global::new(__scope, this)
        }},
    };

    if args
        .iter()
        .any(|(_, Argument { spread, .. })| spread.is_present())
    {
        let casts = args.iter().map(|(ident, Argument { cast, spread })| {
            let name = ident.to_string();
            if spread.is_present() {
                let err = format!("failed to serialize item in argument {name:?}");
                let var = cast.into_v8_local("arg", "__scope");
                quote! {
                    for arg in #ident {
                        let arg = #var.context(#err).context(#ctx)?;
                        let arg = v8::Global::new(__scope, arg);
                        __args.push(arg);
                    }
                }
            } else {
                let err = format!("failed to serialize argument {name:?}");
                let var = cast.into_v8_local(&name, "__scope");
                quote! {
                    let #ident = #var.context(#err).context(#ctx)?;
                    let #ident = v8::Global::new(__scope, #ident);
                    __args.push(#ident);
                }
            }
        });

        let casts = quote! {{
            #scope
            let mut __args = Vec::new();
            __args.push(#this);
            #(#casts)*
            __args
        }};

        (casts, Arity::Variadic)
    } else {
        let casts = args.iter().map(|(ident, Argument { cast, .. })| {
            let name = ident.to_string();
            let err = format!("failed to serialize argument {name:?}");
            let var = cast.into_v8_local(&name, "__scope");
            quote! {
                let #ident = #var.context(#err).context(#ctx)?;
                let #ident = v8::Global::new(__scope, #ident);
            }
        });

        let names = args.iter().map(|(name, _)| name);

        let casts = quote! {{
            #scope
            #(#casts)*
            [#this, #(#names),*]
        }};

        (casts, Arity::Fixed(args.len() + 1))
    }
}
