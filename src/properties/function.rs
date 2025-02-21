use darling::{error::Accumulator, util::SpannedValue, FromMeta, Result};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, FnArg, Generics, Ident, Pat, Receiver, ReturnType,
    Signature, Token, Type,
};
use tap::{Pipe, Tap};

use crate::util::{Feature, FeatureName, NonFatalErrors};

use super::{
    getter, property_key, return_type, self_arg, unwrap_v8_local, Argument, Constructor, FnColor,
    Function, This, TypeCast,
};

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
    fn_color: Option<TokenStream>,
    async_call: TokenStream,
    await_call: TokenStream,
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
            fn_color: errors.handle(FnColor::Async.only::<Function>(sig)),
            async_call: quote! { async },
            await_call: quote! { .await },
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
            fn_color: errors.handle(FnColor::Sync.only::<Constructor>(sig)),
            async_call: quote! {},
            await_call: quote! {},
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
        async_call,
        await_call,
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

    let arguments = BindArgs::new(this, &arguments, quote! { _rt }, quote! { self }, &err_ctx);

    let get_func = {
        let data = quote! { v8::Global<v8::Function> };
        let getter = getter(&name, TypeCast::V8, &data);
        quote! {{
            #getter
            getter(scope, this)
                .context("failed to get function object")?
        }}
    };

    let get_bind = {
        let prop = SpannedValue::new("bind", Span::call_site());
        let data = quote! { v8::Global<v8::Function> };
        let getter = getter(&prop, TypeCast::V8, &data);
        quote! {{
            #getter
            getter(scope, this)
                .context("failed to get Function.property.bind from function")?
        }}
    };

    let bind_fn = {
        let args_ty = arguments.to_type();
        let args_v8_local = arguments.to_v8_local(quote! { args });
        let unwrap = unwrap_v8_local("callable");
        quote! {
            fn bind<'a, T>(
                scope: &mut v8::HandleScope<'a>,
                this: T,
                args: #args_ty,
            ) -> Result<v8::Global<v8::Function>>
            where
                T: TryInto<v8::Local<'a, v8::Object>>,
                T::Error: std::error::Error + Send + Sync + 'static,
            {
                let func = #get_func;
                let bind = {
                    let this = v8::Local::new(scope, &func);
                    #get_bind
                };
                let bind = v8::Local::new(scope, bind);
                let func = v8::Local::new(scope, func);
                let func = Into::into(func);
                let args = #args_v8_local;
                let scope = &mut v8::TryCatch::new(scope);
                let callable = bind.call(scope, func, &args);
                let callable = #unwrap;
                let callable = callable.try_cast()?;
                Ok(v8::Global::new(scope, callable))
            }
        }
    };

    let call_fn = {
        let call = if ctor {
            let unwrap = unwrap_v8_local("retval");
            quote! {
                let scope = &mut _rt.handle_scope();
                let scope = &mut v8::TryCatch::new(scope);
                let callable = v8::Local::<v8::Function>::new(scope, callable);
                let retval = callable.new_instance(scope, &[]);
                let retval = #unwrap;
                let retval = retval.cast::<v8::Value>();
                Ok(v8::Global::new(scope, retval))
            }
        } else {
            quote! {
                let future = _rt.call_with_args(&callable, &[]);
                _rt.run_event_loop(Default::default()).await?;
                future.await
            }
        };
        quote! {
            fn call(
                _rt: &mut JsRuntime,
                callable: v8::Global<v8::Function>
            ) -> Result<v8::Global<v8::Value>> {
                #call
            }
        }
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

    let casts = arguments.v8_globals;

    let impl_ = quote! {
        #fn_color fn #ident <#params> (
            #self_arg,
            #inputs
            _rt: &mut JsRuntime,
        ) -> Result<#return_ty>
        #where_clause
        {
            #[inline(always)]
            #bind_fn

            #[inline(always)]
            #async_call #call_fn

            let args = #casts;

            let callable = {
                let scope = &mut _rt.handle_scope();
                let this = AsRef::<v8::Global<_>>::as_ref(self);
                let this = v8::Local::new(scope, this);
                bind(scope, this, args)
                    .context(#err_ctx)?
            };

            let retval = call(_rt, callable)
                #await_call
                .context(#err_ctx)?;

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

#[derive(Debug)]
struct BindArgs {
    v8_globals: TokenStream,
    kind: BoundArgs,
}

#[derive(Debug, Clone, Copy)]
enum BoundArgs {
    Vec,
    Array(usize),
}

impl BindArgs {
    fn new<T>(this: This, args: &[(Ident, Argument)], rt: T, self_: T, ctx: &str) -> Self
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

            let v8_globals = quote! {{
                #scope
                let mut __args = Vec::new();
                __args.push(#this);
                #(#casts)*
                __args
            }};

            let kind = BoundArgs::Vec;

            Self { v8_globals, kind }
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

            let v8_globals = quote! {{
                #scope
                #(#casts)*
                [#this, #(#names),*]
            }};

            let kind = BoundArgs::Array(args.len() + 1);

            Self { v8_globals, kind }
        }
    }

    fn to_v8_local(&self, ident: impl ToTokens) -> TokenStream {
        match self.kind {
            BoundArgs::Vec => quote! {
                #ident
                    .iter()
                    .map(|arg| v8::Local::new(scope, arg))
                    .collect::<Vec<_>>()
            },
            BoundArgs::Array(_) => quote! {
                #ident.map(|arg| v8::Local::new(scope, arg))
            },
        }
    }

    fn to_type(&self) -> TokenStream {
        match self.kind {
            BoundArgs::Vec => quote! {
                Vec<v8::Global<v8::Value>>
            },
            BoundArgs::Array(len) => quote! {
                [v8::Global<v8::Value>; #len]
            },
        }
    }
}
