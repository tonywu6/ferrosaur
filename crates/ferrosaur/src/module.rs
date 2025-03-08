use darling::{Error, FromDeriveInput, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, Parser},
    Attribute, DeriveInput, Ident, Visibility,
};

use crate::{
    util::{
        inner_mod_name,
        positional::Positional,
        use_prelude,
        v8::snippets::{impl_as_ref_inner, impl_global_cast, impl_to_v8},
        FatalErrors, NoGenerics,
    },
    FastString, ImportMetaUrl, Module, ModuleOptions,
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

    let Module(Positional {
        head: import,
        rest: ModuleOptions { url, fast },
    }) = module;

    let uses = quote! {
        #[allow(unused)]
        use super::*;
        #use_prelude

        #[allow(unused)]
        use deno_core::{
            convert::ToV8,
            anyhow::{Context, Result}, ascii_str_include, v8,
            FastStaticString, FastString,
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

    let preloaded = match fast {
        Some(FastString::Fast | FastString::FastUnsafeDebug) => quote! {
            Ok((Self::module_url()?, FastString::from(Self::MODULE_SRC)))
        },
        None => quote! {
            Ok((Self::module_url()?, FastString::from_static(Self::MODULE_SRC)))
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
        pub fn module_url() -> Result<ModuleSpecifier> {
            #fn_url
            url().context("failed to build module url")
        }
    };

    let impl_as_ref = impl_as_ref_inner(&item_ty, &ident);

    let inner_ty = quote! { v8::Object };

    let impl_to_v8 = impl_to_v8(&inner_ty, &ident);
    let impl_global_cast = impl_global_cast(&inner_ty);

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

                pub async fn main_module_init(rt: &mut JsRuntime) -> Result<Self> {
                    let id = rt
                        .load_main_es_module_from_code(&Self::module_url()?, Self::MODULE_SRC)
                        .await?;
                    Self::mod_evaluate(rt, id).await
                }

                pub async fn side_module_init(rt: &mut JsRuntime) -> Result<Self> {
                    let id = rt
                        .load_side_es_module_from_code(&Self::module_url()?, Self::MODULE_SRC)
                        .await?;
                    Self::mod_evaluate(rt, id).await
                }

                pub fn preloaded() -> Result<(ModuleSpecifier, FastString)> {
                    #preloaded
                }

                #[inline(always)]
                async fn mod_evaluate(rt: &mut JsRuntime, id: ModuleId) -> Result<Self> {
                    Ok(Self({
                        rt.mod_evaluate(id).await?;
                        rt.get_module_namespace(id)?
                    }))
                }

                #impl_global_cast
            }

            #impl_as_ref
            #impl_to_v8
        }
    })
}
