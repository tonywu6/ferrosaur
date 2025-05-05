pub use deno_core::{self, JsRuntime};

use anyhow::Result;
use deno_core::RuntimeOptions;
use deno_web::TimersPermission;

mod globals;

pub fn deno() -> Result<JsRuntime> {
    with_options(Default::default())
}

pub fn with_options(options: RuntimeOptions) -> Result<JsRuntime> {
    Ok(JsRuntime::try_new(RuntimeOptions {
        extensions: vec![
            deno_console::deno_console::init(),
            deno_webidl::deno_webidl::init(),
            deno_url::deno_url::init(),
            deno_web::deno_web::init::<Permissions>(Default::default(), None),
            test_fixture::init(),
        ],
        ..options
    })?)
}

deno_core::extension!(
    test_fixture,
    deps = [deno_web],
    ops = [globals::op_example_read_file],
    esm_entry_point = "ext:globals.js",
    esm = ["ext:globals.js" = "src/globals.js"]
);

struct Permissions;

impl TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}
