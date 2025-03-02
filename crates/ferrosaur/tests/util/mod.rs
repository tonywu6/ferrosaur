use std::{path::Path, rc::Rc};

use anyhow::Result;
use deno_core::{JsRuntime, RuntimeOptions};
use deno_web::TimersPermission;
use tap::Tap;

use crate::compile::modules;

deno_core::extension!(
    test_fixture,
    deps = [deno_web],
    esm_entry_point = "ext:globals.js",
    esm = ["ext:globals.js" = "tests/js/globals.js"]
);

pub fn deno() -> Result<JsRuntime> {
    Ok(JsRuntime::try_new(RuntimeOptions {
        module_loader: Some(Rc::new(modules()?)),
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
