use darling::{
    ast::NestedMeta, error::Accumulator, util::path_to_string, Error, FromGenerics, FromMeta,
    Result,
};
use heck::ToSnakeCase;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parser, punctuated::Punctuated, spanned::Spanned, token::Paren, Attribute, Expr,
    Generics, Ident, Lit, Meta, Path, PathSegment, Token, VisRestricted, Visibility,
};
use tap::{Conv, Pipe, Tap};

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

pub trait NonFatalErrors<T> {
    #[allow(unused)]
    fn non_fatal(self, errors: &mut Accumulator) -> T;
}

impl<T> NonFatalErrors<T> for (T, Option<Error>) {
    fn non_fatal(self, errors: &mut Accumulator) -> T {
        let (ok, err) = self;
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Feature<T>(pub T);

pub trait FeatureName: Sized {
    const PREFIX: &str;

    fn unit() -> Result<Self>;

    fn test(meta: &Meta) -> Result<()> {
        let name = path_to_string(meta.path());
        if Self::PREFIX == name {
            Ok(())
        } else {
            format!("unexpected name `{name}`, expected `{}`", Self::PREFIX)
                .pipe(Error::custom)
                .with_span(meta.path())
                .pipe(Err)
        }
    }

    fn error<T: std::fmt::Display>(msg: T) -> Error {
        Error::custom(format!("({}) {msg}", Self::PREFIX))
    }
}

impl<T> FromMeta for Feature<T>
where
    T: FromMeta + FeatureName,
{
    fn from_meta(item: &Meta) -> Result<Self> {
        T::test(item)?;
        match item {
            Meta::Path(_) => Self::from_word(),
            item => Ok(Self(T::from_meta(item)?)),
        }
    }

    fn from_word() -> Result<Self> {
        Ok(Self(T::unit()?))
    }

    fn from_nested_meta(item: &NestedMeta) -> Result<Self> {
        Ok(Self(T::from_nested_meta(item)?))
    }

    fn from_none() -> Option<Self> {
        Some(Self(T::from_none()?))
    }

    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        Ok(Self(T::from_list(items)?))
    }

    fn from_value(value: &Lit) -> Result<Self> {
        Ok(Self(T::from_value(value)?))
    }

    fn from_expr(expr: &Expr) -> Result<Self> {
        Ok(Self(T::from_expr(expr)?))
    }

    fn from_char(value: char) -> Result<Self> {
        Ok(Self(T::from_char(value)?))
    }

    fn from_string(value: &str) -> Result<Self> {
        Ok(Self(T::from_string(value)?))
    }

    fn from_bool(value: bool) -> Result<Self> {
        Ok(Self(T::from_bool(value)?))
    }
}

impl<T> Feature<T>
where
    T: FromMeta + FeatureName,
{
    pub fn collect(attrs: Vec<Attribute>) -> ((Vec<Self>, Vec<Attribute>), Option<Error>) {
        let mut errors = Error::accumulator();
        let mut items = Vec::new();
        let attrs = attrs
            .into_iter()
            .filter_map(|attr| {
                if T::test(&attr.meta).is_ok() {
                    if let Some(item) = errors.handle(Self::from_meta(&attr.meta)) {
                        items.push(item);
                        None
                    } else {
                        Some(attr)
                    }
                } else {
                    Some(attr)
                }
            })
            .collect();
        ((items, attrs), errors.finish().err())
    }
}

pub trait FeatureEnum: FeatureName {
    const PREFIXES: &[&str];
}

impl<T> Feature<T>
where
    T: FromMeta + FeatureEnum,
{
    pub fn exactly_one(attrs: Vec<Attribute>, span: Span) -> Result<(Self, Vec<Attribute>)> {
        let ((items, attrs), error) = Self::collect(attrs);
        if let Some(error) = error {
            return Err(error);
        };
        match items.len() {
            1 => Ok((items.into_iter().next().unwrap(), attrs)),
            n => {
                let span = if n == 0 {
                    span
                } else {
                    quote! { #(#attrs)* }.span()
                };
                let choices = T::PREFIXES
                    .iter()
                    .map(|c| format!("#[{}({c})]", T::PREFIX))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("expected exactly one of {}", choices)
                    .pipe(T::error)
                    .with_span(&span)
                    .pipe(Err)
            }
        }
    }

    pub fn parse_macro_attribute(attr: TokenStream) -> Result<Self> {
        format_ident!("{}", T::PREFIX)
            .pipe(|name| quote! { #[#name(#attr)] })
            .pipe(|tokens| Attribute::parse_outer.parse2(tokens))?
            .pipe(|attrs| Self::exactly_one(attrs, attr.span()))
            .pipe(|result| Ok(result?.0))
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

pub fn use_prelude() -> TokenStream {
    quote! {
        extern crate alloc as _alloc;
        #[allow(unused)]
        use ::core::{
            convert::{AsRef, From, Into},
            default::Default,
            marker::{Send, Sync},
            option::Option::{self, None, Some},
            result::Result::{Err, Ok},
        };
        #[allow(unused)]
        use _alloc::vec::Vec;
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
