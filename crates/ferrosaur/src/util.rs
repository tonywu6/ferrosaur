use darling::{error::Accumulator, Error, FromGenerics, Result};
use heck::ToSnakeCase;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Paren, Block, FnArg, GenericParam, Generics,
    Ident, ItemImpl, ItemTrait, LifetimeParam, Pat, PatIdent, PatType, Path, PathArguments,
    PathSegment, Receiver, ReturnType, Token, Type, TypeParam, TypePath, VisRestricted, Visibility,
    WhereClause,
};
use tap::{Conv, Pipe, Tap};

pub mod flag;
pub mod function;
pub mod interface;
pub mod property;
pub mod string;
pub mod unary;
pub mod v8;

use self::{
    flag::{FlagEnum, FlagLike, FlagName},
    string::StringLike,
    unary::Unary,
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

pub trait RecoverableErrors<T> {
    fn and_recover(self, errors: &mut Accumulator) -> T;
}

impl<T> RecoverableErrors<T> for Caveat<T> {
    fn and_recover(self, errors: &mut Accumulator) -> T {
        let Caveat(ok, err) = self;
        if let Some(err) = err {
            errors.push(err);
        }
        ok
    }
}

pub trait ErrorLocation {
    fn error_at<E: FlagEnum, F: FlagName>(self) -> Self;
}

impl<T> ErrorLocation for Result<T> {
    fn error_at<E: FlagEnum, F: FlagName>(self) -> Self {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.at(format!("#[{}({})]", E::PREFIX, F::PREFIX))),
        }
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

#[derive(Debug, Clone)]
pub struct MergeGenerics<'a> {
    pub outer: &'a Generics,
    pub lifetimes: Vec<TokenStream>,
    pub types: Vec<TokenStream>,
    pub bounds: Vec<TokenStream>,
}

impl MergeGenerics<'_> {
    pub fn params(&self) -> TokenStream {
        Punctuated::<TokenStream, Token![,]>::new()
            .tap_mut(|p| p.extend(self.lifetimes.iter().cloned()))
            .tap_mut(|p| p.extend(self.outer.lifetimes().map(|lt| quote! { #lt })))
            .tap_mut(|p| p.extend(self.types.iter().cloned()))
            .tap_mut(|p| p.extend(self.outer.type_params().map(|ty| quote! { #ty })))
            .tap_mut(|p| p.extend(self.outer.const_params().map(|c| quote! { #c })))
            .to_token_stream()
    }

    pub fn bounds(&self) -> TokenStream {
        Punctuated::<TokenStream, Token![,]>::new()
            .tap_mut(|p| p.extend(self.bounds.iter().cloned()))
            .tap_mut(|p| {
                if let Some(clauses) = &self.outer.where_clause {
                    p.extend(clauses.predicates.iter().map(|c| c.to_token_stream()))
                }
            })
            .to_token_stream()
    }

    pub fn arguments(&self) -> TokenStream {
        Punctuated::<TokenStream, Token![,]>::new()
            .tap_mut(|p| p.extend(self.lifetimes.iter().cloned()))
            .tap_mut(|p| {
                p.extend(self.outer.lifetimes().map(|lt| {
                    let lt = &lt.lifetime;
                    quote! { #lt }
                }))
            })
            .tap_mut(|p| p.extend(self.types.iter().cloned()))
            .tap_mut(|p| {
                p.extend(self.outer.type_params().map(|ty| {
                    let ty = &ty.ident;
                    quote! { #ty }
                }))
            })
            .tap_mut(|p| {
                p.extend(self.outer.const_params().map(|c| {
                    let c = &c.ident;
                    quote! { #c }
                }))
            })
            .to_token_stream()
    }

    pub fn phantom_fields(&self) -> TokenStream {
        let fields = self.outer.params.iter().filter_map(|p| match p {
            GenericParam::Lifetime(LifetimeParam { lifetime, .. }) => {
                let name = format_ident!("_lifetime_{}", lifetime.ident);
                Some(quote! { #name: ::core::marker::PhantomData<&#lifetime ()> })
            }
            GenericParam::Type(TypeParam { ident, .. }) => {
                let name = format_ident!("_type_{}", ident.to_string().to_lowercase());
                Some(quote! { #name: ::core::marker::PhantomData<#ident> })
            }
            GenericParam::Const(_) => None,
        });
        quote! { #(#fields,)* }
    }

    pub fn phantom_init(&self) -> TokenStream {
        let fields = self.outer.params.iter().filter_map(|p| match p {
            GenericParam::Lifetime(LifetimeParam { lifetime, .. }) => {
                let name = format_ident!("_lifetime_{}", lifetime.ident);
                Some(quote! { #name: ::core::marker::PhantomData })
            }
            GenericParam::Type(TypeParam { ident, .. }) => {
                let name = format_ident!("_type_{}", ident.to_string().to_lowercase());
                Some(quote! { #name: ::core::marker::PhantomData })
            }
            GenericParam::Const(_) => None,
        });
        quote! { #(#fields,)* }
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

pub fn only_inherent_impl(item: &ItemImpl) -> Result<()> {
    let mut errors = Error::accumulator();

    if item.defaultness.is_some() {
        Error::custom("impl cannot be `default`")
            .with_span(&item.defaultness)
            .pipe(|e| errors.push(e));
    }

    if item.unsafety.is_some() {
        Error::custom("impl cannot be `unsafe`")
            .with_span(&item.unsafety)
            .pipe(|e| errors.push(e));
    }

    if let Some((_, ty, _)) = &item.trait_ {
        Error::custom("cannot be a trait impl")
            .with_span(ty)
            .pipe(|e| errors.push(e));
    }

    errors.finish()
}

pub fn only_regular_trait(item: &ItemTrait) -> Result<()> {
    let mut errors = Error::accumulator();

    if item.unsafety.is_some() {
        Error::custom("trait cannot be `unsafe`")
            .with_span(&item.unsafety)
            .pipe(|e| errors.push(e));
    }

    if item.auto_token.is_some() {
        Error::custom("trait cannot be `auto`")
            .with_span(&item.auto_token)
            .pipe(|e| errors.push(e));
    }

    errors.finish()
}

pub fn no_default_fn(defaultness: Option<Token![default]>) -> Result<()> {
    if let Some(span) = defaultness {
        Error::custom("fn cannot be `default` here")
            .with_span(&span)
            .pipe(Err)
    } else {
        Ok(())
    }
}

pub fn no_fn_body(block: Option<Block>) -> Result<()> {
    if let Some(block) = block {
        if !block.stmts.is_empty() {
            Error::custom("macro ignores fn body\nchange this to {}")
                .with_span(&block)
                .pipe(Err)
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

pub fn expect_self_arg<'a>(
    inputs: &'a Punctuated<FnArg, Token![,]>,
    ident: &Ident,
) -> Result<&'a Receiver> {
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
            .with_span(ident)
            .pipe(Err),
    }
}

pub fn only_pat_ident(arg: &FnArg) -> Result<&Ident> {
    let ident = match arg {
        FnArg::Typed(PatType { pat, .. }) => match &**pat {
            Pat::Ident(PatIdent {
                ident,
                by_ref: None,
                mutability: None,
                subpat: None,
                ..
            }) => Some(ident),
            _ => None,
        },
        _ => None,
    };
    if let Some(ident) = ident {
        Ok(ident)
    } else {
        Err(Error::custom("expected an identifier").with_span(arg))
    }
}

pub fn only_explicit_return_type(output: &ReturnType, ident: &Ident) -> Result<()> {
    if matches!(output, ReturnType::Default) {
        Error::custom("must have an explicit return type")
            .with_span(&ident)
            .pipe(Err)
    } else {
        Ok(())
    }
}

pub fn empty_where_clause() -> WhereClause {
    WhereClause {
        where_token: Token![where](Span::call_site()),
        predicates: Default::default(),
    }
}

pub fn type_ident(ident: Ident) -> Type {
    PathSegment {
        ident,
        arguments: PathArguments::None,
    }
    .pipe(|path| Path {
        leading_colon: None,
        segments: std::iter::once(path).collect(),
    })
    .pipe(|path| TypePath { qself: None, path })
    .pipe(Type::Path)
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

#[allow(
    non_camel_case_types,
    reason = "this is used like a unit value in quote! {}"
)]
pub struct use_prelude;

impl ToTokens for use_prelude {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! {
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
        });
    }
}

#[allow(
    non_camel_case_types,
    reason = "this is used like a unit value in quote! {}"
)]
pub struct use_deno;

impl ToTokens for use_deno {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! {
            #[allow(unused)]
            use deno_core::{
                anyhow::{anyhow, Context, Result},
                ascii_str,
                convert::{FromV8, ToV8},
                error::JsError,
                serde_v8, v8, FastString, JsRuntime,
            };
        });
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
