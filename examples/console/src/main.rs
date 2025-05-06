// This example shows the gist of _ferrosaur_:
//
// - You can derive Rust types to represent JavaScript values.
// - You can derive Rust implementations to represent JavaScript interfaces.
// - You can compose these types and implementations to express JavaScript APIs of
//   arbitrary shapes and complexities.
//
// To run this example, run:
//
//     cargo run --package example-console

// ## Getting started

// Everything starts with the `js` macro:

use ferrosaur::js;

// ## Getting `globalThis`

// Use [`#[js(global_this)]`][js-global_this] to derive a [newtype] struct that will
// hold a reference to [`globalThis`]:

#[js(global_this)]
struct Global;
// (this doesn't need to be named "Global")

// ## Naming JavaScript values

// Use [`#[js(value)]`][js-value] to derive a newtype struct that will hold an
// arbitrary JavaScript value:

/// the `Deno` namespace
#[js(value)]
struct Deno;

// ## Declaring JavaScript APIs

// Now that you have these "value types," use [`#[js(interface)]`][js-interface] to describe them:

// ### Properties

// Use [`#[js(prop)]`][js-prop] to derive a Rust function that will access a corresponding
// JavaScript property:

#[js(interface)]
impl Deno {
    #[js(prop)]
    fn pid(&self) -> serde<u32> {}
    // access the `Deno.pid` property
}

// ```ts
// // if we were writing TypeScript, this would be:
// interface Deno {
//   readonly pid: number;
// }
// ```

// ### Data serialization

// Thanks to [`serde_v8`], Rust types that implement [`Serialize`]/[`DeserializeOwned`]
// can be passed to/from JavaScript. To indicate that a type `T` should be converted
// using [`serde_v8`], write it as `serde<T>`, like the `serde<u32>` above.

// ### Functions

// Use [`#[js(func)]`][js-func] to derive a Rust function that will call a corresponding
// JavaScript function:

#[js(interface)]
impl Global {
    /// <https://docs.deno.com/api/web/~/btoa>
    #[js(func)]
    fn btoa(&self, to_encode: serde<&str>) -> serde<String> {}
}

// ### Preserving object identities

// But what if we want more than just the data? What if we would like to keep
// JavaScript objects and values around so that we can use them later? Here comes
// the fun part:

// Thanks to the [`FromV8`]/[`ToV8`] traits, any Rust type derived using this crate
// can also be passed from/to JavaScript (as can any type that implements those traits).
// This is the default conversion mechanism if you _don't_ specify types as `serde<T>`.

// ## Getting to `console.log`

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

// ```ts
// // if we were writing TypeScript, this would be:
// declare global {
//   namespace globalThis {
//     var console: Console;
//   }
// }
// interface Console {
//   log(message: string): void;
// }
// ```

// ## Running everything

// Enough declaring! Let's finally run everything:

#[tokio::main]
async fn main() -> Result<()> {
    let rt: &mut JsRuntime = &mut deno()?;
    // all APIs derived using this crate require a &mut JsRuntime
    // here I'm using a preconfigured runtime, see examples/_runtime for more

    let global = Global::new(rt);
    let console = global.console(rt)?;
    let encoded = global.btoa(r#"{"alg":"HS256"}"#, rt)?;
    console.log(&encoded, rt)?;

    Ok(())
}

// This will run the following equivalent JavaScript:

// ```js
// let console = globalThis.console;
// let encoded = globalThis.btoa('{"alg":"HS256"}');
// console.log(encoded);
// ```

// ---
//
// <details>
//   <summary>Additional setup code for this example</summary>

use anyhow::Result;

use example_runtime::{deno, deno_core, JsRuntime};

// </details>

// [`globalThis`]:          http://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/globalThis
// [ferrosaur]:             /docs/src/index.md
// [js-func]:               /docs/src/reference/interface/func.md
// [js-interface]:          /docs/src/reference/interface.md
// [js-global_this]:        /docs/src/reference/global-this.md
// [js-prop]:               /docs/src/reference/interface/prop.md
// [js-value]:              /docs/src/reference/value.md
// [newtype]:               https://doc.rust-lang.org/rust-by-example/generics/new_types.html
