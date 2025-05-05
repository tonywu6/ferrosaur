use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};
use proc_macro2::TokenStream;
use quote::quote;

fn main() -> Result<()> {
    println!("cargo::rerun-if-changed=build.ts");

    let built = Command::new("deno")
        .args(["run", "--allow-all", "build.ts"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to bundle JavaScript resources, is Deno installed?")?
        .wait()?
        .success();

    if !built {
        bail!("failed to bundle JavaScript resources")
    }

    let lib_dts = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../node_modules/typescript/lib")
        .read_dir()?
        .filter_map(|entry| -> Option<TokenStream> {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?;
            if !name.ends_with(".d.ts") {
                return None;
            }
            let text = std::fs::read_to_string(&path).ok()?;
            let path = path.to_string_lossy();
            println!("cargo::rerun-if-changed={path}");
            if text.is_ascii() {
                Some(quote! {
                    #[allow(long_running_const_eval)]
                    (ascii_str!(#name), FastString::from(ascii_str_include!(#path)))
                })
            } else {
                Some(quote! {
                    (ascii_str!(#name), FastString::from_static(include_str!(#path)))
                })
            }
        });

    let lib_dts = quote! {
        use ::example_runtime::deno_core::{
            anyhow::Result, v8,
            FastString, ascii_str, ascii_str_include,
        };

        pub fn dts(scope: &mut v8::HandleScope) -> Result<v8::Global<v8::Object>> {
            let obj = v8::Object::new(scope);
            let files = [ #(#lib_dts),* ];
            for (lib, dts) in files {
                let lib = lib.v8_string(scope)?;
                let dts = dts.v8_string(scope)?;
                obj.set(scope, lib.into(), dts.into()).unwrap();
            }
            Ok(v8::Global::new(scope, obj))
        }
    };

    std::fs::write(
        Path::new(&std::env::var("OUT_DIR")?).join("lib.dts.rs"),
        lib_dts.to_string(),
    )?;

    Ok(())
}
