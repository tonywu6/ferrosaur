use darling::{
    util::{Flag, SpannedValue},
    Error, FromMeta, Result,
};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, FnArg, Generics, Ident, Pat, ReturnType, Signature,
    Token,
};
use tap::{Pipe, Tap};

use crate::{properties::self_arg, util::FromMetaEnum};

use super::{getter, property_key, return_type, unwrap_v8_local, FnColor, TypeCast};

#[derive(Debug, Default, Clone, FromMeta)]
pub struct Function {
    name: Option<SpannedValue<String>>,
    this: Option<SpannedValue<This>>,
    constructor: Flag,
    #[darling(default)]
    cast: TypeCast,
}

#[derive(Debug, Default, Clone, Copy, FromMeta)]
enum This {
    #[darling(rename = "self")]
    #[default]
    Self_,
    #[darling(rename = "undefined")]
    Undefined,
}

#[derive(Debug, Clone, Copy, FromMeta)]
enum Argument {
    Argument {
        #[darling(default)]
        cast: TypeCast,
        spread: Flag,
    },
}

impl Default for Argument {
    fn default() -> Self {
        Argument::Argument {
            cast: Default::default(),
            spread: Default::default(),
        }
    }
}

impl FromMetaEnum for Argument {
    fn test(name: &str) -> bool {
        name == "argument"
    }

    fn from_unit(_: &str) -> Result<Self> {
        "expected at least 1 option\nto use defaults, remove this attribute"
            .pipe(Error::custom)
            .pipe(Err)
    }
}

pub fn impl_function(func: Function, sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Error::accumulator();

    let Function {
        name,
        cast,
        this,
        constructor,
    } = func;

    let fn_color = if constructor.is_present() {
        errors.handle(FnColor::Sync.only(&sig))
    } else {
        errors.handle(FnColor::Async.only(&sig))
    };

    let span = sig.span();

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

    let fn_name = property_key(&ident, &name);

    let err_ctx = if constructor.is_present() {
        format!("failed to construct {:?}", fn_name.as_str())
    } else {
        format!("failed to call function {:?}", fn_name.as_str())
    };

    let self_arg = errors.handle(self_arg(&inputs, span)).cloned();

    let return_ty = return_type(&output);

    errors.handle(cast.option_check(&output, &ident));

    let mut arguments = Vec::<(Ident, Argument)>::new();

    let inputs = inputs
        .into_iter()
        .skip(1)
        .map(|arg| {
            let FnArg::Typed(arg) = arg else { return arg };

            let (options, arg) = {
                let mut arg = arg;

                let (options, attrs) = Argument::filter_attrs(arg.attrs, &mut errors);
                arg.attrs = attrs;

                if options.len() > 1 {
                    Error::custom("more than one #[argument] specified")
                        .with_span(&arg)
                        .pipe(|e| errors.push(e))
                }
                let options = options.into_iter().next().unwrap_or_default();

                (options, arg)
            };

            let Pat::Ident(name) = &*arg.pat else {
                Error::custom("patterns are not supported here")
                    .with_span(&arg.pat)
                    .pipe(|e| errors.push(e));
                return FnArg::Typed(arg);
            };

            if let Some((sub, _)) = &name.subpat {
                Error::custom("subpatterns are not supported here")
                    .with_span(sub)
                    .pipe(|e| errors.push(e));
            }

            arguments.push((name.ident.clone(), options));

            FnArg::Typed(arg)
        })
        .collect::<Punctuated<FnArg, Token![,]>>();

    let arguments = arguments;

    let this = match (this.as_deref(), constructor.is_present()) {
        (Some(This::Self_), true) => {
            let span = this.map(|s| s.span()).unwrap();
            "must not specify `this` when `constructor` is specified\nremove `this`"
                .pipe(Error::custom)
                .with_span(&span)
                .pipe(|e| errors.push(e));
            This::Undefined
        }
        (Some(This::Self_), false) => This::Self_,
        (Some(This::Undefined), _) => This::Undefined,
        (None, true) => This::Undefined,
        (None, false) => This::Self_,
    };

    let arguments = BindArgs::new(this, &arguments, quote! { _rt }, quote! { self }, &err_ctx);

    let get_func = {
        let data = quote! { v8::Global<v8::Function> };
        let getter = getter(&fn_name, TypeCast::V8, &data);
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
        let call = if constructor.is_present() {
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

    let (async_call, await_call) = if constructor.is_present() {
        (quote! {}, quote! {})
    } else {
        (quote! { async }, quote! { .await })
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
            .any(|(_, Argument::Argument { spread, .. })| spread.is_present())
        {
            let casts = args
                .iter()
                .map(|(ident, Argument::Argument { cast, spread })| {
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
            let casts = args.iter().map(|(ident, Argument::Argument { cast, .. })| {
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
