use darling::{Error, FromDeriveInput, Result};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, Parser},
    Attribute, DeriveInput, Ident,
};

use crate::{
    util::{
        inner_mod_name, use_deno, use_prelude,
        v8_conv_impl::{
            impl_as_ref_inner, impl_from_inner, impl_from_v8, impl_into_inner, impl_to_v8,
        },
        FatalErrors, NoGenerics,
    },
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

    let Value { of: inner_ty } = value;

    let outer_ty = match inner_ty {
        InnerType(ref ty) => quote! {
            v8::Global<#ty>
        },
    };

    let impl_from = impl_from_inner(&outer_ty, &ident);
    let impl_into = impl_into_inner(&outer_ty, &ident);
    let impl_as_ref = impl_as_ref_inner(&outer_ty, &ident);
    let impl_from_v8 = impl_from_v8(&inner_ty.0.to_token_stream(), &ident);
    let impl_to_v8 = impl_to_v8(&inner_ty.0.to_token_stream(), &ident);

    let inner_mod = inner_mod_name("value", &ident);

    let use_prelude = use_prelude();
    let use_deno = use_deno();

    errors.finish()?;

    Ok(quote! {
        #[doc(inline)]
        #vis use #inner_mod::#ident;

        #[doc(hidden)]
        mod #inner_mod {
            use super::*;
            #use_prelude
            #use_deno

            #(#attrs)*
            pub struct #ident(#outer_ty);

            #impl_from
            #impl_into
            #impl_as_ref
            #impl_from_v8
            #impl_to_v8
        }
    })
}
