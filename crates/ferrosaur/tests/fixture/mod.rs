use std::{path::Path, rc::Rc};

use anyhow::Result;
use deno_core::{v8, JsRuntime, RuntimeOptions};
use tap::Tap;

pub fn deno() -> Result<JsRuntime> {
    example_runtime::with_options(RuntimeOptions {
        module_loader: Some(Rc::new(items::modules()?)),
        ..Default::default()
    })
}

#[path = "../../examples/fixture.rs"]
pub mod items;

#[allow(unused, reason = "used in doctests")]
pub fn eval_value<T>(code: &'static str, rt: &mut JsRuntime) -> Result<T>
where
    v8::Global<v8::Value>: Into<T>,
{
    Ok(rt.execute_script("[eval]", code)?.into())
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
