use darling::{Error, FromDeriveInput, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, Parser},
    Attribute, DeriveInput, Ident, Visibility,
};

use crate::{
    util::{
        inner_mod_name, use_prelude,
        v8_conv_impl::{impl_as_ref_inner, impl_from_inner, impl_to_v8},
        FatalErrors, NoGenerics, Unary,
    },
    FastString, ImportMetaUrl, Module,
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
        import: Unary(import),
        url,
        side_module,
        fast,
    } = module;

    let use_prelude = use_prelude();

    let uses = quote! {
        #[allow(unused)]
        use super::*;
        #use_prelude

        #[allow(unused)]
        use deno_core::{
            convert::ToV8,
            anyhow::{Context, Result}, ascii_str_include, v8, FastStaticString,
            JsRuntime, ModuleId, ModuleSpecifier,
        };
    };

    let item_ty = quote! { v8::Global<v8::Object> };

    let item = quote! {
        #(#attrs)*
        pub struct #ident(#item_ty);
    };

    let const_module_src = match fast {
        Some(FastString::FastUnsafeDebug) => {
            let this_crate = format_ident!("{}", env!("CARGO_CRATE_NAME"));
            quote! {
                pub const MODULE_SRC: FastStaticString =
                    ::#this_crate::unsafe_include_fast_string!(#import);
            }
        }
        Some(FastString::Fast) => {
            quote! {
                #[allow(long_running_const_eval)]
                pub const MODULE_SRC: FastStaticString = ascii_str_include!(#import);
            }
        }
        None => {
            quote! {
                pub const MODULE_SRC: &str = include_str!(#import);
            }
        }
    };

    let preload_ty = match fast {
        Some(FastString::Fast | FastString::FastUnsafeDebug) => quote! {
            Result<(ModuleSpecifier, FastStaticString)>
        },
        None => quote! {
            Result<(ModuleSpecifier, &'static str)>
        },
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
                Ok(#fn_url)
            }
        },
        ImportMetaUrl::Cwd => quote! {
            #[inline(always)]
            fn url() -> Result<ModuleSpecifier> {
                let file = #fn_url;
                let name = file.path().replace('/', "-");
                let path = std::env::current_dir()?.join(name);
                Ok(ModuleSpecifier::from_file_path(path).unwrap())
            }
        },
        ImportMetaUrl::Url(url) => quote! {
            #[inline(always)]
            fn url() -> Result<ModuleSpecifier> {
                Ok(#url.parse()?)
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

    let load_module_from_code = if side_module.is_present() {
        quote! { load_side_es_module_from_code }
    } else {
        quote! { load_main_es_module_from_code }
    };

    let load_module_from_loader = if side_module.is_present() {
        quote! { load_side_es_module }
    } else {
        quote! { load_main_es_module }
    };

    let impl_as_ref = impl_as_ref_inner(&item_ty, &ident);
    let impl_from = impl_from_inner(&item_ty, &ident);
    let impl_to_v8 = impl_to_v8(&quote! { v8::Object }, &ident);

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
                    Ok((Self::url()?, Self::MODULE_SRC))
                }

                #[inline(always)]
                async fn evaluate(rt: &mut JsRuntime, id: ModuleId) -> Result<Self> {
                    Ok(Self({
                        rt.mod_evaluate(id).await?;
                        rt.get_module_namespace(id)?
                    }))
                }
            }

            #impl_as_ref
            #impl_from
            #impl_to_v8
        }
    })
}
