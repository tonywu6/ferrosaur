use std::path::Path;

use anyhow::Result;
use bat::PrettyPrinter;
use example_runtime::{
    deno,
    deno_core::{self, serde},
};
use ferrosaur::js;

#[js(module(import("../dist/main.js"), fast(unsafe_debug)))]
struct BlankSpace;

#[js(interface)]
impl BlankSpace {
    #[js(func(name(default)))]
    fn blank_space<S: serde::Serialize>(&self, src: serde<S>) -> serde<String> {}
}

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut deno(BlankSpace::module_url()?)?.js_runtime;

    let ts = BlankSpace::new(rt).await?;

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ts/src/main.ts");
    let source = std::fs::read_to_string(source)?;

    let js = ts.blank_space(source, rt)?;

    PrettyPrinter::new()
        .input_from_bytes(js.as_bytes())
        .language("javascript")
        .theme("GitHub")
        .print()?;

    Ok(())
}
