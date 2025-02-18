use std::borrow::Cow;

use darling::{util::SpannedValue, Error, FromMeta, Result};
use heck::ToLowerCamelCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, Block, FnArg, Generics, Ident, ImplItem, ImplItemFn,
    ItemImpl, Receiver, ReturnType, Signature, Token, Type,
};
use tap::Pipe;

use crate::util::{use_prelude, BailWithErrors, FromMetaEnum};

mod function;
mod property;

use self::{
    function::{impl_function, Function},
    property::{impl_property, Property},
};

#[derive(Debug, Default, Clone, Copy, FromMeta)]
#[darling(rename_all = "lowercase")]
enum TypeCast {
    #[default]
    Serde,
    V8,
    #[darling(rename = "v8::nullish")]
    V8Nullish,
}

pub fn properties(_: TokenStream, item: ItemImpl) -> Result<TokenStream> {
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

    let use_prelude = use_prelude();

    errors.finish()?;

    Ok(quote! {
        const _: () = {
            #use_prelude

            #[allow(unused)]
            use deno_core::{
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
    .or_bail_with(errors)?;

    let (impl_, errors) = match item {
        PropertyType::Property(prop) => impl_property(prop, sig),
        PropertyType::Function(func) => impl_function(func, sig),
    }
    .or_bail_with(errors)?;

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

impl FnColor {
    fn only(self, sig: &Signature) -> Result<TokenStream> {
        let mut errors = Error::accumulator();

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

        match self {
            FnColor::Sync => {
                if sig.asyncness.is_some() {
                    Error::custom("fn cannot be `async` here")
                        .with_span(&sig.asyncness)
                        .pipe(|e| errors.push(e));
                }
            }
            FnColor::Async => {
                if sig.asyncness.is_none() {
                    Error::custom("fn is required to be `async` here")
                        .with_span(&sig.fn_token)
                        .pipe(|e| errors.push(e));
                }
            }
        }

        errors.finish_with(match self {
            FnColor::Sync => quote! {},
            FnColor::Async => {
                let async_ = sig.asyncness;
                quote! { #async_ }
            }
        })
    }
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
        Some(FnArg::Typed(ty)) => Error::custom("must have `&self` as the first argument")
            .with_span(ty)
            .pipe(Err),
        None => Error::custom("missing `&self` as the first argument")
            .with_span(&sig)
            .pipe(Err),
    }
}

fn return_type(ty: &ReturnType) -> TokenStream {
    match ty {
        ReturnType::Type(_, ty) => quote! { #ty },
        ReturnType::Default => quote! { () },
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

impl TypeCast {
    #[allow(clippy::wrong_self_convention)]
    fn from_v8_local<K: AsRef<str>>(&self, name: K, scope: &'static str) -> TokenStream {
        let ident = format_ident!("{}", name.as_ref());
        let handle = format_ident!("{scope}");
        match self {
            TypeCast::Serde => quote! {{
                serde_v8::from_v8(#handle, #ident)
            }},
            TypeCast::V8 => quote! {{
                match #ident.try_cast() {
                    Ok(value) => Ok(v8::Global::new(#handle, value)),
                    Err(error) => Err(error)
                }
            }},
            TypeCast::V8Nullish => {
                let inner = TypeCast::V8.from_v8_local(name.as_ref(), scope);
                quote! {{
                    if #ident.is_null_or_undefined() {
                        Ok(None)
                    } else {
                        #inner.map(Some)
                    }
                }}
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn into_v8_local<K: AsRef<str>>(&self, name: K, scope: &'static str) -> TokenStream {
        let ident = format_ident!("{}", name.as_ref());
        let handle = format_ident!("{scope}");
        match self {
            TypeCast::Serde => quote! {{
                serde_v8::to_v8(#handle, #ident)
            }},
            TypeCast::V8 => quote! {{
                v8::Local::new(#handle, #ident).try_cast()
            }},
            TypeCast::V8Nullish => {
                let inner = TypeCast::V8.into_v8_local(name.as_ref(), scope);
                quote! {{
                    match #ident {
                        None => v8::null(#handle).try_cast(),
                        Some(#ident) => #inner
                    }
                }}
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn into_return_value<K: AsRef<str>>(&self, name: K) -> TokenStream {
        let ident = format_ident!("{}", name.as_ref());
        match self {
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

    fn option_check<S: Spanned>(&self, ty: &ReturnType, ident: S) -> Result<()> {
        fn may_be_option(ty: &Type) -> bool {
            match ty {
                Type::Path(path) => match path.path.segments.last() {
                    None => false,
                    Some(name) => name.ident == "Option",
                },
                Type::Paren(..) => false, // now why would you do that
                _ => false,
            }
        }
        match (self, ty) {
            (TypeCast::V8, ReturnType::Type(_, ty)) => {
                if may_be_option(ty) {
                    [
                        "this will always return Some(...) because of `cast(v8)`",
                        "to check `null` and `undefined` at runtime, use `cast(v8::nullish)`",
                        "otherwise, remove `Option`",
                    ]
                    .join("\n")
                    .pipe(Error::custom)
                    .with_span(ty)
                    .pipe(Err)
                } else {
                    Ok(())
                }
            }
            (TypeCast::V8Nullish, ReturnType::Type(_, ty)) => {
                if may_be_option(ty) {
                    Ok(())
                } else {
                    "`cast(v8::nullish)` requires `Option<...>` as a return type"
                        .pipe(Error::custom)
                        .with_span(&ty)
                        .pipe(Err)
                }
            }
            (TypeCast::V8 | TypeCast::V8Nullish, ReturnType::Default) => {
                "must specify a return type when `cast(v8...)` is specified"
                    .pipe(Error::custom)
                    .with_span(&ident)
                    .pipe(Err)
            }
            (TypeCast::Serde, _) => Ok(()),
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

fn getter<K, T>(prop: &SpannedValue<K>, cast: TypeCast, ty: T) -> TokenStream
where
    K: AsRef<str>,
    T: ToTokens,
{
    let unwrap_data = unwrap_v8_local("data");
    let from_data = cast.from_v8_local("data", "scope");
    let return_ok = cast.into_return_value("data");
    let prop_name = embedded_key(prop);
    quote! {
        #[inline(always)]
        fn getter<'a, T>(
            scope: &mut v8::HandleScope<'a>,
            this: T,
        ) -> Result<#ty>
        where
            T: TryInto<v8::Local<'a, v8::Object>>,
            T::Error: ::std::error::Error + Send + Sync + 'static
        {
            let scope = &mut v8::TryCatch::new(scope);
            let this = TryInto::try_into(this)
                .context("failed to cast `self` as a v8::Object")?;
            let prop = #prop_name.v8_string(scope)?;
            let prop = Into::into(prop);
            let data = this.get(scope, prop);
            let data = #unwrap_data;
            let data = #from_data
                .context("failed to convert from v8 value")?;
            #return_ok
        }
    }
}

fn setter<K, T>(prop: &SpannedValue<K>, cast: TypeCast, ty: T) -> TokenStream
where
    K: AsRef<str>,
    T: ToTokens,
{
    let prop_name = embedded_key(prop);
    let into_data = cast.into_v8_local("data", "scope");
    quote! {
        #[inline(always)]
        fn setter<'a, T>(
            scope: &mut v8::HandleScope<'a>,
            this: T,
            data: #ty
        ) -> Result<()>
        where
            T: TryInto<v8::Local<'a, v8::Object>>,
            T::Error: ::std::error::Error + Send + Sync + 'static
        {
            let data = #into_data
                .context("failed to convert into v8 value")?;
            let this = TryInto::try_into(this)
                .context("failed to cast `self` as a v8::Object")?;
            let prop = #prop_name.v8_string(scope)?;
            let prop = Into::into(prop);
            this.set(scope, prop, data);
            Ok(())
        }
    }
}
