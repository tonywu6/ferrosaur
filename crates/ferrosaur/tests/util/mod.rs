use std::{path::Path, rc::Rc};

use anyhow::Result;
use deno_core::{v8, JsRuntime, RuntimeOptions};
use deno_web::TimersPermission;
use tap::Tap;

#[path = "../../examples/compile.rs"]
pub mod compile;

deno_core::extension!(
    test_fixture,
    deps = [deno_web],
    esm_entry_point = "ext:globals.js",
    esm = ["ext:globals.js" = "examples/js/globals.js"]
);

pub fn deno() -> Result<JsRuntime> {
    Ok(JsRuntime::try_new(RuntimeOptions {
        module_loader: Some(Rc::new(compile::modules()?)),
        extensions: vec![
            deno_console::deno_console::init_ops_and_esm(),
            deno_webidl::deno_webidl::init_ops_and_esm(),
            deno_url::deno_url::init_ops_and_esm(),
            deno_web::deno_web::init_ops_and_esm::<Permissions>(Default::default(), None),
            test_fixture::init_ops_and_esm(),
        ],
        ..Default::default()
    })?)
}

#[allow(unused, reason = "used in doctests")]
pub fn eval_value<T>(code: &'static str, rt: &mut JsRuntime) -> Result<T>
where
    v8::Global<v8::Value>: Into<T>,
{
    Ok(rt.execute_script("[eval]", code)?.into())
}

struct Permissions;

impl TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}

#[allow(unused)]
pub fn with_portable_snapshot<T: FnOnce()>(cb: T, module: &'static str) -> Result<()> {
    let snapshot_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots");

    let path = module
        .split("::")
        .fold(snapshot_dir, |dir, path| dir.join(path));

    insta::Settings::clone_current()
        .tap_mut(|settings| settings.set_snapshot_path(path))
        .tap_mut(|settings| settings.set_prepend_module_to_snapshot(false))
        .bind(cb);

    Ok(())
}
