use darling::{util::Flag, Error, FromDeriveInput, FromMeta, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Ident, Visibility};

use crate::util::{inner_mod_name, DenoCorePath, FromMetaList, NoGenerics, ReturnWithErrors};

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(supports(struct_unit), forward_attrs)]
struct ModuleStruct {
    ident: Ident,
    vis: Visibility,
    attrs: Vec<Attribute>,
    #[allow(unused)]
    generics: NoGenerics,
}

#[derive(Debug, Clone)]
struct Import {
    import: String,
    options: Options,
}

#[derive(Debug, Clone, FromMeta)]
struct Options {
    fast: Flag,
    side: Flag,
    #[darling(default)]
    url: ImportMetaUrl,
    #[darling(default)]
    deno_core: DenoCorePath,
}

#[derive(Debug, Default, Clone, Copy, FromMeta)]
#[darling(rename_all = "lowercase")]
enum ImportMetaUrl {
    #[default]
    Preserve,
    Cwd,
}

impl FromMeta for Import {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        let (import, options) = items
            .split_first()
            .ok_or_else(|| Error::custom("must specify the file path to import"))?;

        let mut errors = Error::accumulator();

        let import = errors.handle(String::from_nested_meta(import));
        let options = errors.handle(Options::from_list(options));

        errors.finish()?;

        let import = import.unwrap();
        let options = options.unwrap();

        Ok(Self { import, options })
    }
}

pub fn module(attr: TokenStream, item: &DeriveInput) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (item, errors) = ModuleStruct::from_derive_input(item).or_return_with(errors)?;

    let (attr, errors) = Import::from_meta_list(attr).or_return_with(errors)?;

    let ModuleStruct {
        ident, vis, attrs, ..
    } = item;

    let Import {
        import,
        options:
            Options {
                fast,
                side,
                url,
                deno_core,
            },
    } = attr;

    let uses = quote! {
        use #deno_core::{
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

            #[automatically_derived]
            impl ::core::convert::AsRef<v8::Global<v8::Object>> for #ident {
                fn as_ref(&self) -> &v8::Global<v8::Object> {
                    &self.0
                }
            }

            #[automatically_derived]
            impl ::core::convert::From<#ident> for v8::Global<v8::Object> {
                fn from(value: #ident) -> Self {
                    value.0
                }
            }
        }
    })
}
