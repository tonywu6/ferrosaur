use anyhow::{Context, Result};
use deno_core::{JsRuntime, RuntimeOptions};
use deno_web::TimersPermission;
use tap::Tap;

deno_core::extension!(
    test_fixture,
    deps = [deno_web],
    esm_entry_point = "ext:globals.js",
    esm = ["ext:globals.js" = "tests/js/globals.js"]
);

pub async fn deno() -> Result<JsRuntime> {
    Ok(JsRuntime::new(RuntimeOptions {
        extensions: vec![
            deno_console::deno_console::init_ops_and_esm(),
            deno_webidl::deno_webidl::init_ops_and_esm(),
            deno_url::deno_url::init_ops_and_esm(),
            deno_web::deno_web::init_ops_and_esm::<Permissions>(Default::default(), None),
            test_fixture::init_ops_and_esm(),
        ],
        ..Default::default()
    }))
}

struct Permissions;

impl TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}

#[allow(unused)]
pub fn with_portable_snapshot<T: FnOnce()>(file_macro: &'static str, cb: T) -> Result<()> {
    let test_file = file_macro.parse::<std::path::PathBuf>()?;

    let test_dir = test_file
        .parent()
        .context("while resolving test directory")?
        .to_str()
        .context("while converting path to string")?;

    insta::Settings::clone_current()
        .tap_mut(|settings| {
            settings.set_snapshot_path(
                test_file
                    .with_extension("")
                    .join("snapshots")
                    .to_str()
                    .unwrap()
                    .strip_prefix(test_dir)
                    .unwrap()
                    .strip_prefix("/")
                    .unwrap(),
            )
        })
        .tap_mut(|settings| settings.set_prepend_module_to_snapshot(false))
        .bind(cb);

    Ok(())
}
