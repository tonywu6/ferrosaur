use darling::{Error, FromDeriveInput, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, Parser},
    Attribute, DeriveInput, Ident, Visibility,
};

use crate::{
    util::{
        inner_mod_name, use_deno, use_prelude,
        v8_conv_impl::{impl_as_ref_inner, impl_global_cast, impl_to_v8},
        FatalErrors, NoGenerics,
    },
    GlobalThis,
};

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(supports(struct_unit), forward_attrs)]
struct GlobalThisStruct {
    ident: Ident,
    vis: Visibility,
    attrs: Vec<Attribute>,
    #[allow(unused)]
    generics: NoGenerics,
}

pub fn global_this(_: GlobalThis, item: TokenStream) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (item, errors) = DeriveInput::parse.parse2(item).or_fatal(errors)?;
    let (item, errors) = GlobalThisStruct::from_derive_input(&item).or_fatal(errors)?;

    let GlobalThisStruct {
        ident, vis, attrs, ..
    } = item;

    let inner_mod = inner_mod_name("global_this", &ident);

    let v8_inner = quote! { v8::Object };
    let v8_outer = quote! { v8::Global<#v8_inner> };

    let impl_as_ref = impl_as_ref_inner(&v8_outer, &ident);
    let impl_to_v8 = impl_to_v8(&v8_inner, &ident);

    let impl_global_cast = impl_global_cast(&v8_inner);

    errors.finish()?;

    Ok(quote! {
        #[doc(inline)]
        #vis use #inner_mod::#ident;

        #[doc(hidden)]
        mod #inner_mod {
            #[allow(unused)]
            use super::*;
            #use_prelude
            #use_deno

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

                #impl_global_cast
            }

            #impl_as_ref
            #impl_to_v8
        }
    })
}
