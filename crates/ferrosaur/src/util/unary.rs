use darling::{ast::NestedMeta, Error, FromMeta, Result};
use syn::{Expr, Lit, Meta};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Unary<T>(pub T);

impl<T: FromMeta> FromMeta for Unary<T> {
    fn from_meta(item: &Meta) -> Result<Self> {
        match item {
            Meta::Path(_) => Self::from_word(),
            Meta::List(list) => {
                let items = NestedMeta::parse_meta_list(list.tokens.clone())?;
                Self::from_list(&items)
            }
            Meta::NameValue(kv) => Self::from_expr(&kv.value),
        }
        .map_err(|e| e.with_span(item))
    }

    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        match items {
            [meta] => match meta {
                NestedMeta::Lit(value) => Ok(Self(T::from_value(value)?)),
                NestedMeta::Meta(item) => Ok(Self(T::from_meta(item)?)),
            },
            [] => Err(Error::too_few_items(1)),
            [..] => Err(Error::too_many_items(1)),
        }
    }

    fn from_nested_meta(item: &NestedMeta) -> Result<Self> {
        Ok(Self(T::from_nested_meta(item)?))
    }

    fn from_bool(value: bool) -> Result<Self> {
        Ok(Self(T::from_bool(value)?))
    }

    fn from_char(value: char) -> Result<Self> {
        Ok(Self(T::from_char(value)?))
    }

    fn from_expr(expr: &Expr) -> Result<Self> {
        Ok(Self(T::from_expr(expr)?))
    }

    fn from_none() -> Option<Self> {
        Some(Self(T::from_none()?))
    }

    fn from_string(value: &str) -> Result<Self> {
        Ok(Self(T::from_string(value)?))
    }

    fn from_value(value: &Lit) -> Result<Self> {
        Ok(Self(T::from_value(value)?))
    }

    fn from_word() -> Result<Self> {
        Ok(Self(T::from_word()?))
    }
}
