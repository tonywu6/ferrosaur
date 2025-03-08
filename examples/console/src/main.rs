//! This example shows the gist of [ferrosaur].
//!
//! To run this example, run
//!
//!     cargo run --package example-console

// Everything starts with the `js` macro:

use ferrosaur::js;

// Use #[js(global_this)] to derive a newtype struct to hold a reference to globalThis:

#[js(global_this)]
struct Global;

// Use #[js(value)] to derive a newtype struct to hold an arbitrary JavaScript value:

/// the `Deno` namespace
#[js(value)]
struct Deno;

// Now that you have these "value types," use #[js(interface)] to describe them:

// Use #[js(prop)] to derive a Rust function that will access a corresponding
// JavaScript property:

#[js(interface)]
impl Deno {
    #[js(prop)]
    fn pid(&self) -> serde<u32> {}
    // access the `Deno.pid` property
}

// Thanks to [serde_v8], Rust types that implement serde::Serialize/Deserialize
// can be passed to/from JavaScript. To indicate that a type `T` should be converted
// using serde_v8, write it as `serde<T>`, like the `serde<u32>` above.
//
// [serde_v8]: https://crates.io/crates/serde_v8

// Use #[js(func)] to derive a Rust function that will call a corresponding
// JavaScript function:

#[js(interface)]
impl Global {
    /// <https://docs.deno.com/api/web/~/btoa>
    #[js(func)]
    fn btoa(&self, to_encode: serde<&str>) -> serde<String> {}
}

// What if we want more than just the data? What if we would like to keep
// JavaScript objects and values around so that we can use them later? Here comes
// the fun part:

// Thanks to the [FromV8]/[ToV8] traits, any Rust type derived using _ferrosaur_
// can also be passed from/to JavaScript (as can any type that implements these traits).
// This is the default conversion mechanism if you don't specify `serde<T>`.
//
// [FromV8]: https://docs.rs/deno_core/0.338.0/deno_core/convert/trait.FromV8.html
// [ToV8]: https://docs.rs/deno_core/0.338.0/deno_core/convert/trait.ToV8.html

// Combining these attributes lets you statically declare JavaScript APIs of
// arbitrary shapes. For example, here's how you declare the existence of `console.log`:

#[js(interface)]
impl Global {
    // there's a `console` on `globalThis` ...
    #[js(prop)]
    fn console(&self) -> Console {}
}

#[js(value)]
struct Console;

#[js(interface)]
impl Console {
    // ... which has a `log` function
    #[js(func)]
    fn log(&self, message: serde<&str>) {}
    // note that we are only allowing a single `&str` message for now
}

// Enough declaring! Let's finally call JavaScript:

#[tokio::main]
async fn main() -> Result<()> {
    // all APIs derived using _ferrosaur_ require a &mut JsRuntime
    // here I'm using a preconfigured runtime, see examples/_runtime for more
    let rt: &mut JsRuntime = &mut js_runtime()?;

    let global = Global::new(rt);

    let console = global.console(rt)?;
    let encoded = global.btoa(r#"{"alg":"HS256"}"#, rt)?;

    console.log(&encoded, rt)?;

    // This is equivalent to the following JavaScript code:
    // globalThis.console.log(globalThis.btoa(`{"alg":"HS256"}`));

    Ok(())
}

use anyhow::Result;
use example_runtime::{
    deno,
    deno_core::{self, JsRuntime},
};

/// See [example_runtime::deno].
fn js_runtime() -> Result<JsRuntime> {
    // MainWorker requires a main module url, although this example doesn't use modules
    let main_module_url = "file:///main.js".parse()?;
    Ok(deno(main_module_url)?.js_runtime)
}
