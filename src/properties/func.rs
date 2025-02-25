use darling::{error::Accumulator, Result};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, Expr, FnArg, Generics, Ident, Pat, PatIdent,
    PatRange, RangeLimits, Receiver, ReturnType, Signature, Token, Type,
};
use tap::{Pipe, Tap};

use crate::util::{
    BindFunction, FlagName, FunctionLength, FunctionThis, NewtypeMeta, NonFatalErrors, PropertyKey,
    V8Conv,
};

use super::{name_or_symbol, property_key, self_arg, Constructor, Function, MaybeAsync};

pub enum Callable {
    Func(Function),
    Ctor(Constructor),
}

struct Callee {
    return_ty: V8Conv,
    this: FunctionThis,
    ctor: bool,
    name: PropertyKey<String>,
    err_ctx: String,
    self_arg: Option<Receiver>,
    fn_color: MaybeAsync,
}

impl Callee {
    fn from_func(
        Function { name, symbol, this }: Function,
        sig: &Signature,
        errors: &mut Accumulator,
    ) -> Self {
        let name = name_or_symbol::<Function>(sig.span(), name.into_inner(), symbol.into_inner())
            .non_fatal(errors);
        let name = property_key(&sig.ident, name);
        let err_ctx = format!("failed to call function {name:?}");
        let return_ty = V8Conv::from_output(sig.output.clone()).non_fatal(errors);
        Self {
            return_ty,
            this,
            ctor: false,
            name,
            err_ctx,
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
        let return_ty = V8Conv::from_output(sig.output.clone()).non_fatal(errors);

        let name = match (class.into_inner().into_inner(), &sig.output) {
            (Some(class), _) => PropertyKey::String(class),

            (None, ReturnType::Type(..)) => {
                let ty = return_ty.as_type();
                let ident = match ty {
                    Type::Path(ty) => {
                        let last = ty.path.segments.last();
                        if let Some(last) = last {
                            if last.arguments.is_none() {
                                Some(&last.ident)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(ident) = ident {
                    PropertyKey::String(ident.to_string())
                } else {
                    "cannot infer class name from return type\nspecify `#[js(new(class(...)))]` instead"
                        .pipe(Constructor::error)
                        .with_span(ty)
                        .pipe(|e| errors.push(e));
                    property_key(&sig.ident, Default::default())
                }
            }

            (None, ReturnType::Default) => {
                "cannot infer class name\nspecify a return type, or use `#[js(new(class(...)))]`"
                    .pipe(Constructor::error)
                    .with_span(&sig.ident)
                    .pipe(|e| errors.push(e));
                property_key(&sig.ident, Default::default())
            }
        };

        let err_ctx = format!("failed to construct {name:?}");

        Self {
            return_ty,
            this: FunctionThis::Self_,
            ctor: true,
            name,
            err_ctx,
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
        return_ty,
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

    let mut arguments = Vec::<Argument>::new();

    let inputs = inputs
        .into_iter()
        .skip(1)
        .map(|arg| {
            let FnArg::Typed(arg) = arg else { return arg };

            let ty = V8Conv::from_type((*arg.ty).clone()).non_fatal(&mut errors);

            match *arg.pat {
                Pat::Ident(ref ident) => {
                    if let Some((sub, _)) = &ident.subpat {
                        Argument::error("subpattern not supported\nremove this")
                            .with_span(sub)
                            .pipe(|e| errors.push(e));
                    }
                    if let Some(ref_) = &ident.by_ref {
                        Argument::error("`ref` not supported\nremove this")
                            .with_span(ref_)
                            .pipe(|e| errors.push(e));
                    }
                    if let Some(mut_) = &ident.mutability {
                        Argument::error("`mut` not supported\nremove this")
                            .with_span(mut_)
                            .pipe(|e| errors.push(e));
                    }
                    let spread = false;
                    let ident = ident.ident.clone();
                    let arg = arg.tap_mut(|arg| arg.ty = ty.to_type().into());
                    arguments.push(Argument { ident, ty, spread });
                    FnArg::Typed(arg)
                }

                Pat::Range(PatRange {
                    start: None,
                    end: Some(ref end),
                    limits: RangeLimits::HalfOpen(..),
                    ..
                }) => match &**end {
                    Expr::Path(path)
                        if path.path.segments.len() == 1
                            && path.path.leading_colon.is_none()
                            && path.qself.is_none() =>
                    {
                        let spread = true;
                        let ident = path.path.segments[0].ident.clone();
                        // let ty = ty.non_null();
                        let arg = arg.tap_mut(|arg| {
                            arg.pat = Pat::Ident(PatIdent {
                                attrs: vec![],
                                by_ref: None,
                                mutability: None,
                                ident: ident.clone(),
                                subpat: None,
                            })
                            .into();
                            arg.ty = ty.to_type().into();
                        });
                        arguments.push(Argument { ident, ty, spread });
                        FnArg::Typed(arg)
                    }
                    expr => {
                        "spread argument should be written as `..name`\nfound extra patterns"
                            .pipe(Argument::error)
                            .with_span(expr)
                            .pipe(|e| errors.push(e));
                        FnArg::Typed(arg)
                    }
                },

                Pat::Range(PatRange {
                    start: Some(ref start),
                    ..
                }) => {
                    "spread argument should be written as `..name`"
                        .pipe(Argument::error)
                        .with_span(start)
                        .pipe(|e| errors.push(e));
                    FnArg::Typed(arg)
                }

                ref pat => {
                    Argument::error("pattern not supported")
                        .with_span(pat)
                        .pipe(|e| errors.push(e));
                    FnArg::Typed(arg)
                }
            }
        })
        .collect::<Punctuated<FnArg, Token![,]>>();

    let arguments = arguments;

    let (casts, length) = scan_args(&arguments, quote! { _rt }, &err_ctx);

    let fn_call = BindFunction {
        name,
        this,
        ctor,
        length,
    }
    .to_function();

    let await_retval = match fn_color {
        MaybeAsync::Async(_) => quote! {{
            let future = _rt.resolve(retval);
            _rt.with_event_loop_promise(future, Default::default())
                .await?
        }},
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
            let from_retval = return_ty.to_cast_from_v8("retval", "scope");
            quote! {{
                let scope = &mut _rt.handle_scope();
                let retval = v8::Local::new(scope, retval);
                let retval = #from_retval
                    .context("failed to convert returned value")
                    .context(#err_ctx)?;
                Ok(retval)
            }}
        }
    };

    let inputs = inputs.tap_mut(|p| {
        if !p.empty_or_trailing() {
            p.push_punct(Token![,](Span::call_site()));
        }
    });

    let return_ty = return_ty.to_type();

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
                let this = ToV8::to_v8(self, scope)?;
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

struct Argument {
    ident: Ident,
    ty: V8Conv,
    spread: bool,
}

impl FlagName for Argument {
    const PREFIX: &'static str = "arg";

    fn unit() -> Result<Self> {
        unreachable!()
    }
}

fn scan_args<T>(args: &[Argument], rt: T, ctx: &str) -> (TokenStream, FunctionLength)
where
    T: ToTokens,
{
    let scope = quote! {
        let __scope = &mut #rt.handle_scope();
    };

    if args.iter().any(|Argument { spread, .. }| *spread) {
        let casts = args.iter().map(|Argument { ident, ty, spread }| {
            let name = ident.to_string();
            if *spread {
                let err = format!("failed to serialize item in argument {name:?}");
                let var = ty.to_cast_into_v8("arg", "__scope");
                quote! {
                    #[allow(for_loops_over_fallibles)]
                    for arg in #ident {
                        let arg = #var.context(#err).context(#ctx)?;
                        let arg = v8::Global::new(__scope, arg);
                        __args.push(arg);
                    }
                }
            } else {
                let err = format!("failed to serialize argument {name:?}");
                let var = ty.to_cast_into_v8(&name, "__scope");
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
            #(#casts)*
            __args
        }};

        (casts, FunctionLength::Variadic)
    } else {
        let casts = args.iter().map(|Argument { ty, ident, .. }| {
            let name = ident.to_string();
            let err = format!("failed to serialize argument {name:?}");
            let var = ty.to_cast_into_v8(&name, "__scope");
            quote! {
                let #ident = #var.context(#err).context(#ctx)?;
                let #ident = v8::Global::new(__scope, #ident);
            }
        });

        let names = args.iter().map(|Argument { ident, .. }| ident);

        let casts = quote! {{
            #scope
            #(#casts)*
            [#(#names),*]
        }};

        (casts, FunctionLength::Fixed(args.len()))
    }
}
