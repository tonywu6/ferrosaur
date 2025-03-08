use anyhow::Result;
use example_runtime::{deno, deno_core};
use ferrosaur::js;

#[js(global_this)]
struct Global;

#[js(interface)]
impl Global {
    /// <https://docs.deno.com/api/web/~/btoa>
    #[js(func)]
    fn btoa(&self, to_encode: serde<&str>) -> serde<String> {}

    /// <https://docs.deno.com/api/web/~/console>
    #[js(prop)]
    fn console(&self) -> Console {}
}

/// <https://docs.deno.com/api/web/~/Console>
#[js(value)]
struct Console;

#[js(interface)]
impl Console {
    /// <https://docs.deno.com/api/web/~/Console#methods_log_11>
    #[js(func)]
    fn log(&self, message: serde<&str>) {}
}

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut deno("file:///main.js".parse()?)?.js_runtime;

    let global = Global::new(rt);

    let console = global.console(rt)?;
    let encoded = global.btoa(r#"{"alg":"HS256"}"#, rt)?;

    console.log(&encoded, rt)?;

    Ok(())
}
