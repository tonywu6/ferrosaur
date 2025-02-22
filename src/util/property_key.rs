use darling::{Error, FromMeta, Result};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Lit, Meta};
use tap::Pipe;

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
