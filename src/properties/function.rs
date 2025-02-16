use darling::{
    util::{Flag, SpannedValue},
    Error, FromMeta, Result,
};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    punctuated::Punctuated, spanned::Spanned, FnArg, Generics, Ident, Pat, ReturnType, Signature,
    Token,
};
use tap::Pipe;

use crate::{
    properties::{into_return_value, self_arg},
    util::FromMetaEnum,
};

use super::{
    cast_from_v8_local, cast_into_v8_local, fn_color, getter, property_key, return_type,
    unwrap_v8_local, FnColor, TypeCast,
};

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
        errors.handle(fn_color(&sig, FnColor::Sync))
    } else {
        errors.handle(fn_color(&sig, FnColor::Async))
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

    let self_arg = errors.handle(self_arg(&inputs, span)).cloned();

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

    let (casts, array) = if arguments.is_empty() {
        let casts = quote! {{ [] as [v8::Global<v8::Value>; 0] }};
        let slice = quote! {{ [this] }};
        (casts, slice)
    } else if arguments
        .iter()
        .any(|(_, Argument::Argument { spread, .. })| spread.is_present())
    {
        let casts = arguments
            .iter()
            .map(|(ident, Argument::Argument { cast, spread })| {
                let name = ident.to_string();
                if spread.is_present() {
                    let err = format!("failed to serialize item in argument {name:?}");
                    let var = cast_into_v8_local("arg", *cast, &err, "__bindgen_scope");
                    quote! {
                        for arg in #ident {
                            let arg = #var;
                            let arg = v8::Global::new(__bindgen_scope, arg);
                            __bindgen_args.push(arg);
                        }
                    }
                } else {
                    let err = format!("failed to serialize argument {name:?}");
                    let var = cast_into_v8_local(&name, *cast, &err, "__bindgen_scope");
                    quote! {
                        let #ident = #var;
                        let #ident = v8::Global::new(__bindgen_scope, #ident);
                        __bindgen_args.push(#ident);
                    }
                }
            });

        let casts = quote! {{
            let mut __bindgen_args = Vec::<v8::Global<v8::Value>>::new();
            let __bindgen_scope = &mut rt.handle_scope();
            #(#casts)*
            __bindgen_args
        }};

        let slice = quote! {{
            extern crate alloc;
            let mut items = alloc::vec![this];
            items.extend(args.iter().map(|arg| v8::Local::new(scope, arg)));
            items
        }};

        (casts, slice)
    } else {
        let casts = arguments
            .iter()
            .map(|(ident, Argument::Argument { cast, .. })| {
                let name = ident.to_string();
                let err = format!("failed to serialize argument {name:?}");
                let var = cast_into_v8_local(&name, *cast, &err, "__bindgen_scope");
                quote! {
                    let #ident = #var;
                    let #ident = v8::Global::new(__bindgen_scope, #ident);
                }
            });

        let names = arguments.iter().map(|(name, _)| name);

        let casts = quote! {{
            let __bindgen_scope = &mut rt.handle_scope();
            #(#casts)*
            [#(#names),*]
        }};

        let array = arguments.iter().enumerate().map(|(rhs, _)| {
            let lhs = rhs + 1;
            quote! { array[#lhs] = v8::Local::new(scope, &args[#rhs]); }
        });

        let len = arguments.len() + 1;

        let array = quote! {{
            let undefined = v8::undefined(scope).cast::<v8::Value>();
            let mut array = [undefined; #len];
            array[0] = this;
            #(#array)*
            array
        }};

        (casts, array)
    };

    let get_func = if constructor.is_present() {
        if let Some(name) = &name {
            "must not specify `name` when `constructor` is specified\nremove `name`"
                .pipe(Error::custom)
                .with_span(&name.span())
                .pipe(|e| errors.push(e));
        }
        quote! {
            let func = this.try_cast::<v8::Function>()
                .context("failed to cast value as a constructor")?;
            v8::Global::new(scope, func)
        }
    } else {
        let prop = property_key(&ident, &name);
        let data = quote! { v8::Global<v8::Function> };
        let getter = getter(&prop, &data, TypeCast::V8);
        let err = format!("failed to get function {:?}", prop.as_str());
        quote! {
            #getter
            getter(scope, this).context(#err)?
        }
    };

    let get_bind = {
        let prop = SpannedValue::new("bind", Span::call_site());
        let data = quote! { v8::Global<v8::Function> };
        let getter = getter(&prop, &data, TypeCast::V8);
        let err = "failed to get Function.property.bind from function";
        quote! {
            #getter
            getter(scope, this).context(#err)?
        }
    };

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

    let receiver = match this {
        This::Self_ => quote! {{
            let this = AsRef::<v8::Global<_>>::as_ref(self);
            let this = v8::Local::new(scope, this);
            this
        }},
        This::Undefined => quote! {
            v8::undefined(scope)
        },
    };

    let callable = {
        let unwrap = unwrap_v8_local("callable");
        quote! {{
            let scope = &mut rt.handle_scope();
            let scope = &mut v8::TryCatch::new(scope);

            let bind = v8::Local::new(scope, bind);
            let func = v8::Local::new(scope, func);
            let func = Into::into(func);
            let this = #receiver;
            let this = Into::into(this);
            let args = #array;

            let callable = bind.call(scope, func, &args);

            let callable = #unwrap;
            let callable = callable.try_cast()?;
            v8::Global::new(scope, callable)
        }}
    };

    let call = if constructor.is_present() {
        let unwrap = unwrap_v8_local("retval");
        quote! {
            let retval = {
                let scope = &mut rt.handle_scope();
                let scope = &mut v8::TryCatch::new(scope);
                let callable = v8::Local::<v8::Function>::new(scope, callable);
                let retval = callable.new_instance(scope, &[]);
                let retval = #unwrap;
                let retval = retval.cast::<v8::Value>();
                v8::Global::new(scope, retval)
            };
        }
    } else {
        quote! {
            let retval = {
                let future = rt.call_with_args(&callable, &[]);
                rt.run_event_loop(Default::default()).await?;
                future.await?
            };
        }
    };

    let return_ty = return_type(&output, cast, &mut errors);

    let retval = match &output {
        ReturnType::Default => {
            quote! {
                let _ = retval;
                Ok(())
            }
        }
        ReturnType::Type(..) => {
            let from_retval =
                cast_from_v8_local("retval", cast, "failed to convert returned value", "scope");
            let into_retval = into_return_value("retval", cast);
            quote! {{
                let scope = &mut rt.handle_scope();
                let retval = v8::Local::new(scope, retval);
                let retval = #from_retval;
                #into_retval
            }}
        }
    };

    let impl_ = quote! {
        #fn_color fn #ident <#params> (
            #self_arg,
            rt: &mut JsRuntime,
            #inputs
        ) -> Result<#return_ty>
        #where_clause
        {
            let args = #casts;

            let func = {
                let scope = &mut rt.handle_scope();
                let this = AsRef::<v8::Global<_>>::as_ref(self);
                let this = v8::Local::new(scope, this);
                #get_func
            };

            let bind = {
                let scope = &mut rt.handle_scope();
                let this = v8::Local::new(scope, &func);
                #get_bind
            };

            let callable = #callable;

            #call

            #retval
        }
    };

    errors.finish()?;

    Ok(vec![impl_])
}
