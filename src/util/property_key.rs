use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

#[derive(Clone)]
pub enum PropertyKey {
    String(String),
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

impl<K: AsRef<str>> From<K> for PropertyKey {
    fn from(value: K) -> Self {
        Self::String(value.as_ref().into())
    }
}

impl std::fmt::Debug for PropertyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => s.fmt(f),
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

impl ToTokens for PropertyKey {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let rendered = match self {
            Self::String(key) => {
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
