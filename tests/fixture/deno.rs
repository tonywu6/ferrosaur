use anyhow::Result;
use deno_core::{JsRuntime, RuntimeOptions};
use deno_web::TimersPermission;

use super::Main;

deno_core::extension!(
    test_fixture,
    deps = [deno_web],
    esm_entry_point = "ext:globals.js",
    esm = ["ext:globals.js" = "tests/fixture/js/globals.js"]
);

pub async fn deno() -> Result<(JsRuntime, Main)> {
    let mut rt = JsRuntime::new(RuntimeOptions {
        extensions: vec![
            deno_console::deno_console::init_ops_and_esm(),
            deno_webidl::deno_webidl::init_ops_and_esm(),
            deno_url::deno_url::init_ops_and_esm(),
            deno_web::deno_web::init_ops_and_esm::<Permissions>(Default::default(), None),
            test_fixture::init_ops_and_esm(),
        ],
        ..Default::default()
    });

    let main = Main::new(&mut rt).await?;

    Ok((rt, main))
}

struct Permissions;

impl TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}
