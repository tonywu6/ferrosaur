use darling::{Error, FromDeriveInput, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, Parser},
    Attribute, DeriveInput, Ident, Visibility,
};

use crate::{
    util::{inner_mod_name, use_prelude, FatalErrors, NoGenerics},
    ImportMetaUrl, Module, ModuleOptions,
};

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(supports(struct_unit), forward_attrs)]
struct ModuleStruct {
    ident: Ident,
    vis: Visibility,
    attrs: Vec<Attribute>,
    #[allow(unused)]
    generics: NoGenerics,
}

pub fn module(module: Module, item: TokenStream) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (item, errors) = DeriveInput::parse.parse2(item).or_fatal(errors)?;
    let (item, errors) = ModuleStruct::from_derive_input(&item).or_fatal(errors)?;

    let ModuleStruct {
        ident, vis, attrs, ..
    } = item;

    let Module {
        import,
        options: ModuleOptions { fast, side, url },
    } = module;

    let use_prelude = use_prelude();

    let uses = quote! {
        use super::*;

        #use_prelude

        #[allow(unused)]
        use deno_core::{
            anyhow::{Context, Result}, ascii_str_include, v8, FastStaticString,
            JsRuntime, ModuleId, ModuleSpecifier,
        };
    };

    let item = quote! {
        #(#attrs)*
        pub struct #ident(v8::Global<v8::Object>);
    };

    let const_module_src = if fast.is_present() {
        quote! {
            pub const MODULE_SRC: FastStaticString = ascii_str_include!(#import);
        }
    } else {
        quote! {
            pub const MODULE_SRC: &str = include_str!(#import);
        }
    };

    let preload_ty = if fast.is_present() {
        quote! {
            Result<(ModuleSpecifier, FastStaticString)>
        }
    } else {
        quote! {
            Result<(ModuleSpecifier, &'static str)>
        }
    };

    let fn_url = quote! {
        "file:///"
            .parse::<ModuleSpecifier>()?
            .join(file!())?
            .join(#import)?
    };

    let fn_url = match url {
        ImportMetaUrl::Preserve => quote! {
            #[inline(always)]
            fn url() -> Result<ModuleSpecifier> {
                Result::Ok(#fn_url)
            }
        },
        ImportMetaUrl::Cwd => quote! {
            #[inline(always)]
            fn url() -> Result<ModuleSpecifier> {
                let file = #fn_url;
                let name = file.path().replace('/', "-");
                let path = std::env::current_dir()?.join(name);
                Result::Ok(ModuleSpecifier::from_file_path(path).unwrap())
            }
        },
    };

    let fn_url = quote! {
        #[inline(always)]
        pub fn url() -> Result<ModuleSpecifier> {
            #fn_url
            url().context("failed to build module url")
        }
    };

    let load_module_from_code = if side.is_present() {
        quote! { load_side_es_module_from_code }
    } else {
        quote! { load_main_es_module_from_code }
    };

    let load_module_from_loader = if side.is_present() {
        quote! { load_side_es_module }
    } else {
        quote! { load_main_es_module }
    };

    let inner_mod = inner_mod_name("module", &ident);

    let reexport = quote! {
        #vis use #inner_mod::#ident;
    };

    errors.finish()?;

    Ok(quote! {
        #[doc(inline)]
        #reexport

        #[doc(hidden)]
        mod #inner_mod {
            #[allow(unused)]
            #uses

            #item

            #[automatically_derived]
            impl #ident {
                #const_module_src

                #fn_url

                pub async fn new(rt: &mut JsRuntime) -> Result<Self> {
                    let id = rt
                        .#load_module_from_code(&Self::url()?, Self::MODULE_SRC)
                        .await?;
                    Self::evaluate(rt, id).await
                }

                pub async fn new_preloaded(rt: &mut JsRuntime) -> Result<Self> {
                    let id = rt.#load_module_from_loader(&Self::url()?).await?;
                    Self::evaluate(rt, id).await
                }

                pub fn preload() -> #preload_ty {
                    Result::Ok((Self::url()?, Self::MODULE_SRC))
                }

                #[inline(always)]
                async fn evaluate(rt: &mut JsRuntime, id: ModuleId) -> Result<Self> {
                    Result::Ok(Self({
                        rt.mod_evaluate(id).await?;
                        rt.get_module_namespace(id)?
                    }))
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
