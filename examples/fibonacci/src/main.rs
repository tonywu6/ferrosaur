use ferrosaur::js;

#[js(module("lib.js"))]
struct Math;

#[js(interface)]
impl Math {
    #[js(func)]
    fn slow_fib(&self, n: serde<usize>) -> serde<usize> {}
}

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut JsRuntime::try_new(Default::default())?;

    let lib = Math::main_module_init(rt).await?;
    let fib = lib.slow_fib(42, rt)?;
    assert_eq!(fib, 267914296);

    Ok(())
}

use anyhow::Result;
use deno_core::JsRuntime;
