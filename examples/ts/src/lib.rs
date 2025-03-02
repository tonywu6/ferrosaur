use anyhow::Result;
use ferrosaur::js;

use example_runtime::deno_core::{self, JsRuntime};

#[js(module(
    "../dist/typescript.js",
    url("npm:typescript"),
    side_module,
    fast(unsafe_debug)
))]
pub struct TypeScript;

#[js(interface)]
pub trait Compiler {
    #[js(func)]
    fn create_program(&self, ..files: serde<Vec<String>>) -> Program {}
}

#[js(value)]
pub struct Program;

#[js(interface)]
impl Program {
    #[js(func)]
    pub fn print_diagnostics(&self, colored: bool) -> serde<String> {}
}

#[js(module("../dist/lib.js", fast))]
pub struct Example;

impl Compiler for Example {}

pub fn inject_env_vars(rt: &mut JsRuntime) -> Result<()> {
    #[js(global_this)]
    struct Global;

    #[js(interface)]
    impl Global {
        #[js(set_index)]
        fn define_object(&self, name: serde<&str>, value: v8::Global<v8::Object>) {}

        #[js(set_index)]
        fn define_string(&self, name: serde<&str>, value: serde<&str>) {}
    }

    let global = Global::new(rt);

    let dts = {
        let scope = &mut rt.handle_scope();
        dts::dts(scope)?
    };

    global.define_object("TYPESCRIPT_LIB", dts, rt)?;
    global.define_string("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"), rt)?;

    Ok(())
}

mod dts {
    include!(concat!(env!("OUT_DIR"), "/lib.dts.rs"));
}
