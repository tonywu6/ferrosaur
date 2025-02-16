use darling::{ast::NestedMeta, util::Flag, Error, FromDeriveInput, FromMeta, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    punctuated::Punctuated, Attribute, DeriveInput, Ident, Meta, Path, PathSegment, Token, Type,
    TypePath,
};
use tap::Pipe;

use crate::util::{
    inner_mod_name, use_prelude, DenoCorePath, FromMetaList, NoGenerics, ReturnWithErrors,
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

#[derive(Debug, Clone, FromMeta)]
struct Options {
    serde: Flag,
    #[darling(default)]
    of: InnerType,
    #[darling(default)]
    deno_core: DenoCorePath,
}

pub fn newtype(attr: TokenStream, item: &DeriveInput) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (item, errors) = ValueStruct::from_derive_input(item).or_return_with(errors)?;

    let (attr, errors) = Options::from_meta_list(attr).or_return_with(errors)?;

    let ValueStruct {
        ident, vis, attrs, ..
    } = item;

    let Options {
        serde,
        of,
        deno_core,
    } = attr;

    let inner_ty = match of {
        InnerType(ty) => quote! {
            v8::Global<#ty>
        },
    };

    let impl_serde = if serde.is_present() {
        quote! {
            #[automatically_derived]
            impl<'de> #deno_core::serde::Deserialize<'de> for #ident {
                fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
                where
                    D: #deno_core::serde::Deserializer<'de>,
                {
                    let value = #deno_core::serde_v8::GlobalValue::deserialize(deserializer)?;
                    Ok(Self(value.v8_value))
                }
            }

            #[automatically_derived]
            impl #deno_core::serde::Serialize for #ident {
                fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where
                    S: #deno_core::serde::Serializer,
                {
                    #deno_core::serde_v8::GlobalValue { v8_value: self.0.clone() }
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
            use #deno_core::v8;

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

#[derive(Debug, Clone)]
struct InnerType(Type);

impl FromMeta for InnerType {
    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        let mut errors = Error::accumulator();
        let item = match items.len() {
            1 => &items[0],
            0 => {
                errors.finish()?;
                return Err(Error::custom("must specify a type"));
            }
            _ => {
                errors.push(Error::custom("must specify exactly 1 type"));
                &items[0]
            }
        };
        let (item, errors) = match item {
            NestedMeta::Lit(..) => {
                Error::custom("unexpected literal, expected a type path").pipe(Err)
            }
            NestedMeta::Meta(item) => match item {
                Meta::List(..) => {
                    Error::custom("unexpected nested list, expected a type path").pipe(Err)
                }
                Meta::NameValue(..) => {
                    Error::custom("unexpected assignment, expected a type path").pipe(Err)
                }
                Meta::Path(path) => TypePath {
                    path: path.clone(),
                    qself: None,
                }
                .pipe(Type::Path)
                .pipe(Ok),
            },
        }
        .map_err(|err| err.with_span(item))
        .or_return_with(errors)?;
        errors.finish()?;
        Ok(Self(item))
    }
}

impl Default for InnerType {
    fn default() -> Self {
        [format_ident!("v8"), format_ident!("Value")]
            .map(PathSegment::from)
            .pipe(Punctuated::<PathSegment, Token![::]>::from_iter)
            .pipe(|segments| Path {
                segments,
                leading_colon: None,
            })
            .pipe(|path| TypePath { path, qself: None })
            .pipe(Type::Path)
            .pipe(Self)
    }
}
