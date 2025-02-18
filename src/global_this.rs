use darling::{Error, FromDeriveInput, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Ident, Visibility};

use crate::util::{inner_mod_name, use_prelude, BailWithErrors, NoGenerics};

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(supports(struct_unit), forward_attrs)]
struct Options {
    ident: Ident,
    vis: Visibility,
    attrs: Vec<Attribute>,
    #[allow(unused)]
    generics: NoGenerics,
}

pub fn global_this(_: TokenStream, item: &DeriveInput) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (item, errors) = Options::from_derive_input(item).or_bail_with(errors)?;

    let Options {
        ident, vis, attrs, ..
    } = item;

    let inner_mod = inner_mod_name("global_this", &ident);

    let use_prelude = use_prelude();

    errors.finish()?;

    Ok(quote! {
        #[doc(inline)]
        #vis use #inner_mod::#ident;

        #[doc(hidden)]
        mod #inner_mod {
            use super::*;

            #use_prelude

            use deno_core::{v8, JsRuntime};

            #(#attrs)*
            pub struct #ident(v8::Global<v8::Object>);

            #[automatically_derived]
            impl #ident {
                pub fn new(rt: &mut JsRuntime) -> Self {
                    let context = rt.main_context();
                    let scope = &mut rt.handle_scope();
                    let context = v8::Local::new(scope, context);
                    let global = context.global(scope);
                    let global = v8::Global::new(scope, global);
                    Self(global)
                }
            }

            #[automatically_derived]
            impl AsRef<v8::Global<v8::Object>> for #ident {
                fn as_ref(&self) -> &v8::Global<v8::Object> {
                    &self.0
                }
            }

            #[automatically_derived]
            impl From<#ident> for v8::Global<v8::Object> {
                fn from(value: #ident) -> Self {
                    value.0
                }
            }
        }
    })
}
