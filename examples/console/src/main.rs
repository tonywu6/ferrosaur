// This example shows the gist of _ferrosaur_:
//
// - You can generate Rust types to represent JavaScript types.
// - You can generate Rust implementations to describe JavaScript interfaces.
// - You can compose these types and implementations to express JavaScript APIs of
//   arbitrary shapes and complexities.
//
// To run this example, run:
//
//     cargo run --package example-console
//
// ---

// Everything starts with the `js` macro:

use ferrosaur::js;

// Use [`#[js(global_this)]`][js-global_this] to derive a newtype struct that will hold a reference to [`globalThis`]:

#[js(global_this)]
struct Global;
// (this doesn't need to be named "Global")

// Use [`#[js(value)]`][js-value] to derive a newtype struct that will hold an arbitrary JavaScript value:

/// the `Deno` namespace
#[js(value)]
struct Deno;

// Now that you have these "value types," use `#[js(interface)]` to describe them:

// Use [`#[js(prop)]`][js-prop] to derive a Rust function that will access a corresponding
// JavaScript property:

#[js(interface)]
impl Deno {
    #[js(prop)]
    fn pid(&self) -> serde<u32> {}
    // access the `Deno.pid` property
}

// Thanks to [`serde_v8`], Rust types that implement [`Serialize`]/[`DeserializeOwned`]
// can be passed to/from JavaScript. To indicate that a type `T` should be converted
// using [`serde_v8`], write it as `serde<T>`, like the `serde<u32>` above.

// Use [`#[js(func)]`][js-func] to derive a Rust function that will call a corresponding
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

// Thanks to the [`FromV8`]/[`ToV8`] traits, any Rust type derived using this crate
// can also be passed from/to JavaScript (as can any type that implements those traits).
// This is the default conversion mechanism if you don't specify `serde<T>`.

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
    // all APIs derived using this crate require a &mut JsRuntime
    // here I'm using a preconfigured runtime, see examples/_runtime for more
    let rt: &mut JsRuntime = &mut js_runtime()?;

    let global = Global::new(rt);
    let console = global.console(rt)?;
    let encoded = global.btoa(r#"{"alg":"HS256"}"#, rt)?;
    console.log(&encoded, rt)?;

    Ok(())
}

// This is equivalent to the following JavaScript code:

// ```js
// globalThis.console.log(globalThis.btoa(`{"alg":"HS256"}`));
// ```

// ---
//
// (Below are some setup code for this example.)

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

// [`DeserializeOwned`]:    deno_core::serde::de::DeserializeOwned
// [`FromV8`]:              deno_core::FromV8
// [`Serialize`]:           deno_core::serde::Serialize
// [`ToV8`]:                deno_core::ToV8
// [`globalThis`]:          http://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/globalThis
// [`serde_v8`]:            deno_core::serde_v8
// [js-func]:               https://tonywu6.github.io/ferrosaur/reference/interface/func
// [js-global_this]:        https://tonywu6.github.io/ferrosaur/reference/global-this
// [js-prop]:               https://tonywu6.github.io/ferrosaur/reference/interface/prop
// [js-value]:              https://tonywu6.github.io/ferrosaur/reference/value
