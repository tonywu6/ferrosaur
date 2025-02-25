use darling::{error::Accumulator, Error, FromGenerics, Result};
use heck::ToSnakeCase;
use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Paren, Generics, Ident, ItemImpl, Path,
    PathSegment, Token, VisRestricted, Visibility,
};
use tap::{Conv, Pipe, Tap};

mod bind_function;
mod flag_like;
mod property_key;
mod string_like;
mod unary;
mod v8_conv;
pub mod v8_conv_impl;

pub use self::{
    bind_function::{BindFunction, FunctionLength, FunctionThis},
    flag_like::{FlagEnum, FlagLike, FlagName},
    property_key::{PropertyKey, WellKnown},
    string_like::StringLike,
    unary::Unary,
    v8_conv::{V8Conv, V8InnerType},
};

pub trait TokenStreamResult {
    fn or_error(self) -> TokenStream;
}

impl TokenStreamResult for Result<TokenStream> {
    fn or_error(self) -> TokenStream {
        self.unwrap_or_else(Error::write_errors)
    }
}

pub trait FatalErrors<T> {
    fn or_fatal(self, errors: Accumulator) -> Result<(T, Accumulator)>;
}

impl<T> FatalErrors<T> for Result<T> {
    fn or_fatal(self, errors: Accumulator) -> Result<(T, Accumulator)> {
        match self {
            Ok(value) => Ok((value, errors)),
            Err(error) => errors
                .tap_mut(|errors| errors.push(error))
                .finish()
                .map(|_| unreachable!()),
        }
    }
}

impl<T> FatalErrors<T> for syn::Result<T> {
    fn or_fatal(self, errors: Accumulator) -> Result<(T, Accumulator)> {
        self.map_err(Error::from).or_fatal(errors)
    }
}

#[derive(Debug)]
pub struct Caveat<T>(pub T, pub Option<Error>);

impl<T> Caveat<T> {
    pub fn into_result(self) -> Result<T> {
        match self.1 {
            None => Ok(self.0),
            Some(e) => Err(e),
        }
    }
}

impl<T> From<(T, Error)> for Caveat<T> {
    fn from((ok, err): (T, Error)) -> Self {
        Self(ok, Some(err))
    }
}

impl<T> From<(T, Option<Error>)> for Caveat<T> {
    fn from((ok, err): (T, Option<Error>)) -> Self {
        Self(ok, err)
    }
}

impl<T> From<T> for Caveat<T> {
    fn from(value: T) -> Self {
        Self(value, None)
    }
}

pub trait NonFatalErrors<T> {
    #[allow(unused)]
    fn non_fatal(self, errors: &mut Accumulator) -> T;
}

impl<T> NonFatalErrors<T> for Caveat<T> {
    fn non_fatal(self, errors: &mut Accumulator) -> T {
        let Caveat(ok, err) = self;
        if let Some(err) = err {
            errors.push(err);
        }
        ok
    }
}

pub trait MergeErrors {
    fn into_one(self) -> Option<Error>;
}

impl MergeErrors for Accumulator {
    fn into_one(self) -> Option<Error> {
        let errors = self.into_inner();
        if errors.is_empty() {
            None
        } else {
            Some(Error::multiple(errors))
        }
    }
}

pub trait NewtypeMeta<T> {
    fn into_inner(self) -> T;
}

impl<T> NewtypeMeta<T> for FlagLike<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> NewtypeMeta<T> for Unary<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> NewtypeMeta<T> for StringLike<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T: NewtypeMeta<U>, U> NewtypeMeta<Option<U>> for Option<T> {
    fn into_inner(self) -> Option<U> {
        self.map(NewtypeMeta::into_inner)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoGenerics;

impl FromGenerics for NoGenerics {
    fn from_generics(generics: &Generics) -> Result<Self> {
        let mut errors = Error::accumulator();

        if !generics.params.is_empty() {
            Error::custom("must not have generics")
                .with_span(&generics.params)
                .pipe(|e| errors.push(e))
        }

        if generics.where_clause.is_some() {
            Error::custom("must not have a where clause")
                .with_span(&generics.where_clause)
                .pipe(|e| errors.push(e));
        }

        errors.finish_with(Self)
    }
}

pub fn inner_mod_name<T: ToTokens>(prefix: &str, item: T) -> Ident {
    fn collect_ident(stream: TokenStream, collector: &mut Vec<String>) {
        for token in stream {
            match token {
                TokenTree::Ident(ident) => collector.push(ident.to_string().to_snake_case()),
                TokenTree::Group(group) => collect_ident(group.stream(), collector),
                _ => {}
            }
        }
    }
    let name = {
        let mut tokens = vec![];
        collect_ident(item.to_token_stream(), &mut tokens);
        tokens
    }
    .join("_");
    format!("__bindgen_{prefix}_{name}")
        .to_lowercase()
        .pipe_as_ref(|name| Ident::new(name, item.span()))
}

pub fn only_inherent_impl<F: FlagName>(item: &ItemImpl) -> Result<()> {
    let mut errors = Accumulator::default();

    if item.defaultness.is_some() {
        F::error("impl cannot be `default`")
            .with_span(&item.defaultness)
            .pipe(|e| errors.push(e));
    }

    if item.unsafety.is_some() {
        F::error("impl cannot be `unsafe`")
            .with_span(&item.unsafety)
            .pipe(|e| errors.push(e));
    }

    if let Some((_, ty, _)) = &item.trait_ {
        F::error("cannot be a trait impl")
            .with_span(ty)
            .pipe(|e| errors.push(e));
    }

    errors.finish()
}

#[allow(unused)]
pub fn pub_in_super(vis: Visibility) -> Visibility {
    match vis {
        Visibility::Public(..) => vis,
        Visibility::Restricted(vis) => {
            let span = vis.span();
            if vis.path.segments.first().map(|s| &s.ident) == Some(&Token![super](span).into()) {
                let VisRestricted {
                    pub_token,
                    paren_token,
                    in_token,
                    path: suffix,
                } = vis;
                let prefix = Token![super](span).conv::<Ident>().conv::<PathSegment>();
                let path = Punctuated::<PathSegment, Token![::]>::new()
                    .tap_mut(|p| p.push(prefix))
                    .tap_mut(|p| p.extend(suffix.segments))
                    .pipe(|segments| Path {
                        segments,
                        leading_colon: None,
                    })
                    .into();
                VisRestricted {
                    pub_token,
                    paren_token,
                    in_token: in_token.unwrap_or_default().pipe(Some),
                    path,
                }
            } else {
                vis
            }
            .pipe(Visibility::Restricted)
        }
        Visibility::Inherited => VisRestricted {
            pub_token: Token![pub](vis.span()),
            in_token: None,
            paren_token: Paren(vis.span()),
            path: Token![super](vis.span()).conv::<Path>().pipe(Box::new),
        }
        .pipe(Visibility::Restricted),
    }
}

pub fn unwrap_v8_local(name: &str) -> TokenStream {
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

pub fn use_prelude() -> TokenStream {
    quote! {
        extern crate alloc as _alloc;
        #[allow(unused)]
        use ::core::{
            convert::{AsRef, From, Infallible, Into},
            default::Default,
            marker::{Send, Sync},
            option::Option::{self, None, Some},
            result::Result::{Err, Ok},
        };
        #[allow(unused)]
        use _alloc::vec::Vec;
    }
}

pub fn use_deno() -> TokenStream {
    quote! {
        #[allow(unused)]
        use deno_core::{
            anyhow::{anyhow, Context, Result},
            ascii_str,
            convert::{FromV8, ToV8},
            error::JsError,
            serde_v8, v8, FastString, JsRuntime,
        };
    }
}

#[allow(unused)]
pub fn debug_docs<T: std::fmt::Debug>(item: T) -> TokenStream {
    let docs = format!("```\n{item:#?}\n```")
        .split('\n')
        .map(|line| {
            let line = format!(" {line}");
            quote! { #[doc = #line] }
        })
        .collect::<Vec<_>>();
    quote! { #(#docs)* }
}
