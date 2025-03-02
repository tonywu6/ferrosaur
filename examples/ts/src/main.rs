use std::path::Path;

use anyhow::Result;
use example_runtime::{
    deno,
    deno_core::{self, JsRuntime},
};
use ferrosaur::js;

#[js(module("../dist/main.js", fast(unsafe_debug)))]
struct TypeScript;

#[js(interface)]
impl TypeScript {
    #[js(func)]
    fn create_program(&self, ..files: serde<Vec<String>>) -> Program {}
}

#[js(value)]
struct Program;

#[js(interface)]
impl Program {
    #[js(func)]
    fn print_diagnostics(&self) -> serde<String> {}
}

#[js(global_this)]
struct Global;

#[js(interface)]
impl Global {
    #[js(set_index)]
    fn define(&self, name: serde<&str>, value: v8::Global<v8::Value>) {}
}

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut deno(TypeScript::module_url()?)?.js_runtime;

    inject_dts(rt)?;

    let ts = TypeScript::new(rt).await?;

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/main.ts")
        .to_string_lossy()
        .into_owned();

    let program = ts.create_program(vec![path], rt)?;

    let errors = program.print_diagnostics(rt)?;

    println!("{errors}");

    Ok(())
}

fn inject_dts(rt: &mut JsRuntime) -> Result<()> {
    let global = Global::new(rt);

    let dts = {
        let scope = &mut rt.handle_scope();
        dts::dts(scope)?
    };

    global.define("__TYPESCRIPT_LIB__", dts, rt)?;

    Ok(())
}

mod dts {
    include!(concat!(env!("OUT_DIR"), "/lib.dts.rs"));
}
