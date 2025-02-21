use darling::{
    ast::NestedMeta, error::Accumulator, util::path_to_string, Error, FromGenerics, FromMeta,
    Result,
};
use heck::ToSnakeCase;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parser, punctuated::Punctuated, spanned::Spanned, token::Paren, Attribute, Expr,
    Generics, Ident, Lit, Meta, Path, PathSegment, ReturnType, Token, Type, VisRestricted,
    Visibility,
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
pub struct Positional<P, T> {
    pub head: P,
    pub rest: T,
}

pub trait FromPositional: Sized {
    fn fallback() -> Result<Self>;
}

impl<P, T> FromMeta for Positional<P, T>
where
    P: FromMeta + FromPositional,
    T: FromMeta,
{
    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        match items.len() {
            0 => Ok(Self {
                head: P::fallback()?,
                rest: T::from_list(&[])?,
            }),
            _ => {
                match T::from_list(items)
                    .and_then(|rest| P::fallback().map(|head| Self { head, rest }))
                {
                    Ok(this) => Ok(this),
                    Err(e1) => match T::from_list(&items[1..]).and_then(|rest| {
                        P::from_nested_meta(&items[0]).map(|head| Self { head, rest })
                    }) {
                        Ok(this) => Ok(this),
                        Err(e2) => Err(Error::multiple(vec![e2, e1])),
                    },
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy, FromMeta)]
#[darling(rename_all = "lowercase")]
pub enum TypeCast {
    V8,
    #[darling(rename = "v8::nullish")]
    V8Nullish,
    #[default]
    Serde,
}

impl TypeCast {
    pub fn option_check<F: FeatureName>(&self, ty: &ReturnType) -> Result<()> {
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
                    .pipe(F::error)
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
                        .pipe(F::error)
                        .with_span(&ty)
                        .pipe(Err)
                }
            }
            (TypeCast::V8 | TypeCast::V8Nullish, ReturnType::Default) => Ok(()),
            (TypeCast::Serde, _) => Ok(()),
        }
    }
}

#[derive(Clone, Copy)]
pub enum PropertyKey<K> {
    String(K),
    Number(f64),
    Symbol(WellKnown),
}

#[derive(Clone, Copy, FromMeta)]
#[darling(rename_all = "camelCase")]
pub enum WellKnown {
    AsyncIterator,
    HasInstance,
    IsConcatSpreadable,
    Iterator,
    Match,
    Replace,
    Search,
    Split,
    ToPrimitive,
    ToStringTag,
    Unscopables,
}

impl FromMeta for PropertyKey<String> {
    fn from_meta(item: &Meta) -> Result<Self> {
        let key = if let Meta::Path(path) = item {
            if path.segments.len() == 2 {
                let head = path.segments.get(0).unwrap();
                let tail = path.segments.get(1).unwrap();
                if head.ident == "Symbol" && head.arguments.is_none() && tail.arguments.is_none() {
                    WellKnown::from_string(&tail.ident.to_string())
                        .map_err(|e| e.with_span(tail))?
                        .pipe(Self::Symbol)
                        .pipe(Some)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        if let Some(key) = key {
            Ok(key)
        } else {
            "property key must be a string, a number, or Symbol::*"
                .pipe(Error::custom)
                .with_span(item)
                .pipe(Err)
        }
    }

    fn from_value(value: &Lit) -> Result<Self> {
        match value {
            Lit::Str(s) => Self::from_string(&s.value()),
            Lit::Char(ch) => Self::from_char(ch.value()),
            Lit::Int(n) => Ok(Self::Number(n.base10_parse()?)),
            Lit::Float(f) => Ok(Self::Number(f.base10_parse()?)),
            _ => Err(Error::unexpected_lit_type(value)),
        }
        .map_err(|e| e.with_span(value))
    }

    fn from_string(value: &str) -> Result<Self> {
        Ok(Self::String(value.into()))
    }

    fn from_char(value: char) -> Result<Self> {
        Ok(Self::String(value.into()))
    }
}

impl<'a> From<&'a str> for PropertyKey<&'a str> {
    fn from(value: &'a str) -> Self {
        Self::String(value)
    }
}

impl<T> std::fmt::Debug for PropertyKey<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => s.fmt(f),
            Self::Number(n) => n.fmt(f),
            Self::Symbol(s) => s.fmt(f),
        }
    }
}

impl std::fmt::Debug for WellKnown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AsyncIterator => f.write_str("[Symbol.asyncIterator]"),
            Self::HasInstance => f.write_str("[Symbol.hasInstance]"),
            Self::IsConcatSpreadable => f.write_str("[Symbol.isConcatSpreadable]"),
            Self::Iterator => f.write_str("[Symbol.iterator]"),
            Self::Match => f.write_str("[Symbol.match]"),
            Self::Replace => f.write_str("[Symbol.replace]"),
            Self::Search => f.write_str("[Symbol.search]"),
            Self::Split => f.write_str("[Symbol.split]"),
            Self::ToPrimitive => f.write_str("[Symbol.toPrimitive]"),
            Self::ToStringTag => f.write_str("[Symbol.toStringTag]"),
            Self::Unscopables => f.write_str("[Symbol.unscopables]"),
        }
    }
}

impl<K: AsRef<str>> ToTokens for PropertyKey<K> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let rendered = match self {
            Self::String(key) => {
                let key = key.as_ref();
                if key.is_ascii() {
                    quote! {
                        ascii_str!(#key).v8_string(scope)?
                    }
                } else {
                    quote! {
                        FastString::from_static(#key).v8_string(scope)?
                    }
                }
            }
            Self::Number(num) => {
                quote! {
                    v8::Number::new(scope, #num)
                }
            }
            Self::Symbol(sym) => match sym {
                WellKnown::AsyncIterator => quote! {
                    v8::Symbol::get_async_iterator(scope)
                },
                WellKnown::HasInstance => quote! {
                    v8::Symbol::get_has_instance(scope)
                },
                WellKnown::IsConcatSpreadable => quote! {
                    v8::Symbol::get_is_concat_spreadable(scope)
                },
                WellKnown::Iterator => quote! {
                    v8::Symbol::get_iterator(scope)
                },
                WellKnown::Match => quote! {
                    v8::Symbol::get_match(scope)
                },
                WellKnown::Replace => quote! {
                    v8::Symbol::get_replace(scope)
                },
                WellKnown::Search => quote! {
                    v8::Symbol::get_search(scope)
                },
                WellKnown::Split => quote! {
                    v8::Symbol::get_split(scope)
                },
                WellKnown::ToPrimitive => quote! {
                    v8::Symbol::get_to_primitive(scope)
                },
                WellKnown::ToStringTag => quote! {
                    v8::Symbol::get_to_string_tag(scope)
                },
                WellKnown::Unscopables => quote! {
                    v8::Symbol::get_unscopables(scope)
                },
            },
        };
        tokens.extend(rendered);
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
