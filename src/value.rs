use darling::{Error, FromDeriveInput, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, Parser},
    Attribute, DeriveInput, Ident,
};

use crate::{
    util::{inner_mod_name, use_prelude, FatalErrors, NoGenerics},
    InnerType, Value,
};

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(supports(struct_unit), forward_attrs)]
struct ValueStruct {
    ident: Ident,
    vis: syn::Visibility,
    attrs: Vec<Attribute>,
    #[allow(unused)]
    generics: NoGenerics,
}

pub fn value(value: Value, item: TokenStream) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (item, errors) = DeriveInput::parse.parse2(item).or_fatal(errors)?;
    let (item, errors) = ValueStruct::from_derive_input(&item).or_fatal(errors)?;

    let ValueStruct {
        ident, vis, attrs, ..
    } = item;

    let Value { serde, of } = value;

    let inner_ty = match of {
        InnerType(ty) => quote! {
            v8::Global<#ty>
        },
    };

    let impl_serde = if serde.is_present() {
        quote! {
            #[automatically_derived]
            impl<'de> deno_core::serde::Deserialize<'de> for #ident {
                fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
                where
                    D: deno_core::serde::Deserializer<'de>,
                {
                    let value = deno_core::serde_v8::GlobalValue::deserialize(deserializer)?;
                    Ok(Self(value.v8_value))
                }
            }

            #[automatically_derived]
            impl deno_core::serde::Serialize for #ident {
                fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where
                    S: deno_core::serde::Serializer,
                {
                    deno_core::serde_v8::GlobalValue { v8_value: self.0.clone() }
                        .serialize(serializer)
                }
            }
        }
    } else {
        quote! {}
    };

    let inner_mod = inner_mod_name("value", &ident);

    let prelude = use_prelude();

    errors.finish()?;

    Ok(quote! {
        #[doc(inline)]
        #vis use #inner_mod::#ident;

        #[doc(hidden)]
        mod #inner_mod {
            use super::*;

            #prelude

            #[allow(unused)]
            use deno_core::v8;

            #(#attrs)*
            pub struct #ident(#inner_ty);

            #[automatically_derived]
            impl From<#inner_ty> for #ident {
                fn from(value: #inner_ty) -> Self {
                    Self(value)
                }
            }

            #[automatically_derived]
            impl From<#ident> for #inner_ty {
                fn from(value: #ident) -> Self {
                    value.0
                }
            }

            #[automatically_derived]
            impl AsRef<#inner_ty> for #ident {
                fn as_ref(&self) -> &#inner_ty {
                    &self.0
                }
            }

            #impl_serde
        }
    })
}
