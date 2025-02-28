use darling::{ast::NestedMeta, util::path_to_string, Error, FromMeta, Result};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse::Parser, spanned::Spanned, Attribute, Expr, Lit, Meta};
use tap::Pipe;

use super::Caveat;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlagLike<T>(pub T);

pub trait FlagName: Sized {
    const PREFIX: &'static str;

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
}

impl<T> FromMeta for FlagLike<T>
where
    T: FromMeta + FlagName,
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

impl<T> FlagLike<T>
where
    T: FromMeta + FlagName,
{
    pub fn collect(attrs: Vec<Attribute>) -> Caveat<(Vec<Self>, Vec<Attribute>)> {
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
        ((items, attrs), errors.finish().err()).into()
    }
}

pub trait FlagEnum: FlagName {
    const PREFIXES: &'static [&'static str];
}

impl<T> FlagLike<T>
where
    T: FromMeta + FlagEnum,
{
    pub fn exactly_one(attrs: Vec<Attribute>, span: Span) -> Result<(Self, Vec<Attribute>)> {
        let (items, attrs) = Self::collect(attrs).into_result()?;
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
                    .pipe(Error::custom)
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
