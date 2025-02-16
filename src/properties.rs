use std::borrow::Cow;

use darling::{error::Accumulator, util::SpannedValue, Error, FromMeta, Result};
use heck::ToLowerCamelCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    punctuated::Punctuated, Block, FnArg, Generics, Ident, ImplItem, ImplItemFn, ItemImpl,
    Receiver, ReturnType, Signature, Token,
};
use tap::Pipe;

use crate::util::{use_prelude, DenoCorePath, FromMetaEnum, FromMetaList, ReturnWithErrors};

mod function;
mod property;

use self::{
    function::{impl_function, Function},
    property::{impl_property, Property},
};

#[derive(Debug, Clone, FromMeta)]
struct Options {
    #[darling(default)]
    deno_core: DenoCorePath,
}

#[derive(Debug, Default, Clone, Copy, FromMeta)]
#[darling(rename_all = "lowercase")]
enum TypeCast {
    #[default]
    Serde,
    V8,
    #[darling(rename = "v8::nullish")]
    V8Nullish,
}

pub fn properties(attr: TokenStream, item: ItemImpl) -> Result<TokenStream> {
    let mut errors = Error::accumulator();

    errors.handle(only_inherent_impl(&item));

    let ItemImpl {
        attrs,
        generics,
        self_ty,
        items,
        ..
    } = item;

    let Generics {
        params,
        where_clause,
        ..
    } = generics;

    let items = items
        .into_iter()
        .filter_map(|item| errors.handle(impl_item(item)))
        .collect::<Vec<_>>();

    let (attr, errors) = Options::from_meta_list(attr).or_return_with(errors)?;

    let Options { deno_core } = attr;

    let prelude = use_prelude();

    errors.finish()?;

    Ok(quote! {
        const _: () = {
            #prelude

            #[allow(unused)]
            use #deno_core::{
                anyhow::{anyhow, Context, Result}, error::JsError,
                ascii_str, serde_v8, v8, FastString, JsRuntime,
            };

            #(#attrs)*
            impl <#params> #self_ty
            #where_clause
            {
                #(#items)*
            }
        };
    })
}

fn impl_item(item: ImplItem) -> Result<TokenStream> {
    let ImplItem::Fn(func) = item else {
        return "#[properties] supports only fn items\nmove this item to another impl block"
            .pipe(Error::custom)
            .with_span(&item)
            .pipe(Err);
    };

    let mut errors = Error::accumulator();

    let ImplItemFn {
        attrs,
        vis,
        defaultness,
        sig,
        block,
    } = func;

    errors.handle(no_fn_body(&block));

    errors.handle(if defaultness.is_some() {
        Error::custom("fn cannot be `default` here")
            .with_span(&defaultness)
            .pipe(Err)
    } else {
        Ok(())
    });

    let (items, attrs) = PropertyType::filter_attrs(attrs, &mut errors);

    let (item, errors) = match items.len() {
        1 => Ok(items.into_iter().next().unwrap()),
        _ => Error::custom("must be either #[property] or #[function]")
            .with_span(&sig)
            .pipe(Err),
    }
    .or_return_with(errors)?;

    let (impl_, errors) = match item {
        PropertyType::Property(prop) => impl_property(prop, sig),
        PropertyType::Function(func) => impl_function(func, sig),
    }
    .or_return_with(errors)?;

    errors.finish()?;

    let impl_ = impl_
        .into_iter()
        .map(|impl_| quote! { #(#attrs)* #vis #impl_ });

    Ok(quote! { #(#impl_)* })
}

#[derive(Debug, Clone, FromMeta)]
enum PropertyType {
    Property(Property),
    Function(Function),
}

impl FromMetaEnum for PropertyType {
    fn test(name: &str) -> bool {
        matches!(name, "property" | "function")
    }

    fn from_unit(name: &str) -> Result<Self> {
        match name {
            "property" => Ok(Self::Property(Default::default())),
            "function" => Ok(Self::Function(Default::default())),
            name => Err(Error::unknown_value(name)),
        }
    }
}

fn only_inherent_impl(item: &ItemImpl) -> Result<()> {
    let mut errors = Error::accumulator();

    if item.defaultness.is_some() {
        Error::custom("#[properties] impl cannot be `default`")
            .with_span(&item.defaultness)
            .pipe(|e| errors.push(e));
    }

    if item.unsafety.is_some() {
        Error::custom("#[properties] impl cannot be `unsafe`")
            .with_span(&item.unsafety)
            .pipe(|e| errors.push(e));
    }

    if let Some((_, ty, _)) = &item.trait_ {
        Error::custom("#[properties] cannot be used on a trait impl")
            .with_span(ty)
            .pipe(|e| errors.push(e));
    }

    errors.finish()
}

fn no_fn_body(block: &Block) -> Result<()> {
    if !block.stmts.is_empty() {
        Error::custom("fn body should be empty")
            .with_span(block)
            .pipe(Err)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum FnColor {
    Sync,
    Async,
}

fn fn_color(sig: &Signature, color: FnColor) -> darling::Result<TokenStream> {
    let mut errors = darling::Error::accumulator();

    macro_rules! deny {
        ($attr:ident, $msg:literal) => {
            if sig.$attr.is_some() {
                darling::Error::custom($msg)
                    .with_span(&sig.$attr)
                    .pipe(|e| errors.push(e));
            }
        };
    }

    deny!(constness, "fn cannot be `const` here");
    deny!(unsafety, "fn cannot be `unsafe` here");
    deny!(abi, "fn cannot be `extern` here");
    deny!(variadic, "fn cannot be variadic here");

    match color {
        FnColor::Sync => {
            if sig.asyncness.is_some() {
                darling::Error::custom("fn cannot be `async` here")
                    .with_span(&sig.asyncness)
                    .pipe(|e| errors.push(e));
            }
        }
        FnColor::Async => {
            if sig.asyncness.is_none() {
                darling::Error::custom("fn is required to be `async` here")
                    .with_span(&sig.fn_token)
                    .pipe(|e| errors.push(e));
            }
        }
    }

    errors.finish_with(match color {
        FnColor::Sync => quote! {},
        FnColor::Async => {
            let async_ = sig.asyncness;
            quote! { #async_ }
        }
    })
}

fn self_arg(inputs: &Punctuated<FnArg, Token![,]>, sig: Span) -> Result<&Receiver> {
    match inputs.first() {
        Some(FnArg::Receiver(recv)) => {
            if recv.reference.is_none() || recv.mutability.is_some() {
                Error::custom("must be `&self`").with_span(recv).pipe(Err)
            } else {
                Ok(recv)
            }
        }
        Some(FnArg::Typed(ty)) => Error::custom("must be `&self`").with_span(ty).pipe(Err),
        None => Error::custom("missing `&self`").with_span(&sig).pipe(Err),
    }
}

fn property_key<'a>(
    src: &Ident,
    alt: &'a Option<SpannedValue<String>>,
) -> Cow<'a, SpannedValue<String>> {
    match alt.as_ref() {
        Some(ident) => Cow::Borrowed(ident),
        None => src
            .to_string()
            .to_lower_camel_case()
            .pipe(|s| SpannedValue::new(s, src.span()))
            .pipe(Cow::Owned),
    }
}

fn embedded_key<K: AsRef<str>>(key: &SpannedValue<K>) -> TokenStream {
    let span = key.span();
    let key = (**key).as_ref();
    let inner = quote_spanned! { span => #key };
    if key.is_ascii() {
        quote! { ascii_str!(#inner) }
    } else {
        quote! { FastString::from_static(#inner) }
    }
}

fn return_type(ty: &ReturnType, cast: TypeCast, errors: &mut Accumulator) -> TokenStream {
    match (ty, cast) {
        (ReturnType::Type(_, ty), TypeCast::V8Nullish) => quote! {
            Option<#ty>
        },
        (ReturnType::Type(_, ty), TypeCast::Serde | TypeCast::V8) => quote! {
            #ty
        },
        (ReturnType::Default, TypeCast::Serde) => quote! {
            ()
        },
        (ReturnType::Default, TypeCast::V8 | TypeCast::V8Nullish) => {
            "must specify a return type when `cast(v8...)` is specified"
                .pipe(Error::custom)
                .pipe(|e| errors.push(e));
            quote! {
                Option<()>
            }
        }
    }
}

fn into_return_value<K: AsRef<str>>(name: K, cast: TypeCast) -> TokenStream {
    let ident = format_ident!("{}", name.as_ref());
    match cast {
        TypeCast::Serde => quote! {
            Ok(#ident)
        },
        TypeCast::V8 => quote! {
            Ok(Into::into(#ident))
        },
        TypeCast::V8Nullish => quote! {
            Ok(#ident.map(Into::into))
        },
    }
}

fn cast_from_v8_local<K: AsRef<str>>(
    name: K,
    cast: TypeCast,
    error: &str,
    scope: &'static str,
) -> TokenStream {
    let ident = format_ident!("{}", name.as_ref());
    let handle = format_ident!("{scope}");
    match cast {
        TypeCast::Serde => quote! {{
            serde_v8::from_v8(#handle, #ident)
                .context(#error)?
        }},
        TypeCast::V8 => quote! {{
            let #ident = #ident.try_cast()
                .context(#error)?;
            let #ident = v8::Global::new(#handle, #ident);
            #ident
        }},
        TypeCast::V8Nullish => {
            let inner = cast_from_v8_local(name.as_ref(), TypeCast::V8, error, scope);
            quote! {{
                if #ident.is_null_or_undefined() {
                    None
                } else {
                    Some(#inner)
                }
            }}
        }
    }
}

fn cast_into_v8_local<K: AsRef<str>>(
    name: K,
    cast: TypeCast,
    error: &str,
    scope: &'static str,
) -> TokenStream {
    let ident = format_ident!("{}", name.as_ref());
    let handle = format_ident!("{scope}");
    match cast {
        TypeCast::Serde => quote! {{
            serde_v8::to_v8(#handle, #ident)
                .context(#error)?
        }},
        TypeCast::V8 => quote! {{
            let #ident = v8::Local::new(#handle, #ident);
            let #ident = #ident.try_cast()
                .context(#error)?;
            #ident
        }},
        TypeCast::V8Nullish => {
            let inner = cast_into_v8_local(name.as_ref(), TypeCast::V8, error, scope);
            quote! {{
                match #ident {
                    None => v8::null(#handle).try_cast()?,
                    Some(#ident) => #inner
                }
            }}
        }
    }
}

fn unwrap_v8_local(name: &str) -> TokenStream {
    let err = format!("{name} is None");
    let name = format_ident!("{name}");
    quote! {{
        let Some(#name) = #name else {
            return if let Some(exception) = scope.exception() {
                Err(JsError::from_v8_exception(scope, exception))?
            } else {
                Err(anyhow!(#err))
            };
        };
        #name
    }}
}

fn getter<K: AsRef<str>>(
    prop: &SpannedValue<K>,
    data: &TokenStream,
    cast: TypeCast,
) -> TokenStream {
    let unwrap_data = unwrap_v8_local("data");
    let from_data = cast_from_v8_local("data", cast, "failed to convert value from v8", "scope");
    let return_ok = into_return_value("data", cast);
    let prop_name = embedded_key(prop);
    quote! {
        #[inline(always)]
        fn getter<'a, T>(
            scope: &mut v8::HandleScope<'a>,
            this: T,
        ) -> Result<#data>
        where
            T: Into<v8::Local<'a, v8::Value>>
        {
            let scope = &mut v8::TryCatch::new(scope);
            let this = Into::into(this);
            let this = v8::Local::new(scope, this);
            let this = this
                .try_cast::<v8::Object>()
                .context("failed to cast `this` as a v8::Object")?;
            let prop = #prop_name.v8_string(scope)?;
            let prop = Into::into(prop);
            let data = this.get(scope, prop);
            let data = #unwrap_data;
            let data = #from_data;
            #return_ok
        }
    }
}

fn setter<K: AsRef<str>>(
    prop: &SpannedValue<K>,
    data: &TokenStream,
    cast: TypeCast,
) -> TokenStream {
    let prop_name = embedded_key(prop);
    let into_data = cast_into_v8_local("data", cast, "failed to prepare value for v8", "scope");
    quote! {
        #[inline(always)]
        fn setter<'a, T>(
            scope: &mut v8::HandleScope<'a>,
            this: T,
            data: #data
        ) -> Result<()>
        where
            T: Into<v8::Local<'a, v8::Value>>,
        {
            let data = #into_data;
            let this = Into::into(this);
            let this = v8::Local::new(scope, this);
            let this = this
                .try_cast::<v8::Object>()
                .context("failed to cast `this` as a v8::Object")?;
            let prop = #prop_name.v8_string(scope)?;
            let prop = Into::into(prop);
            this.set(scope, prop, data);
            Ok(())
        }
    }
}
