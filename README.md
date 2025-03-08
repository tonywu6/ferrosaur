# ferrosaur

<em/>So you use</em> deno_core, <em>and you want to call JavaScript from
Rust.</em>

```javascript
// You have:
// lib.js
export const add = (a, b) => a + b;
```

```rust
// You write:
// lib.rs
#[js(module("lib.js"))]
struct Module;

#[js(interface)]
impl Module {
    #[js(func)]
    fn add(&self, a: serde<f64>, b: serde<f64>) -> serde<f64> {}
}
```

```rust
// You get:
// let rt: &mut JsRuntime;
let module = Module::main_module(rt).await?;
let result = module.add(66.0, 3.0)?;
```

ferrosaur derives structs and functions from attributes (à la [wasm-bindgen])
that you can use with your favorite [`JsRuntime`][JsRuntime].

[JsRuntime]: https://docs.rs/deno_core/0.338.0/deno_core/struct.JsRuntime.html
[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen#example

## Intro

Everything starts with the `js` macro:

```rust
use ferrosaur::js;
```

Use `#[js(global_this)]` to derive a [newtype] struct to hold a reference to
[`globalThis`][globalThis].

```rust
#[js(global_this)]
struct Global;
```

Use `#[js(value)]` to derive a newtype struct to hold an arbitrary JavaScript
value.

```rust
/// the `Deno` namespace
#[js(value)]
struct Deno;
```

Now that you have these "value types," use `#[js(interface)]` to describe them:

Use `#[js(prop)]` to derive a Rust function that will access a corresponding
JavaScript property.

```rust
#[js(interface)]
impl Deno {
    #[js(prop)]
    fn pid(&self) -> serde<u32> {}
    // access the `Deno.pid` property
}
```

Thanks to [`serde_v8`][serde_v8], Rust types that implement
`serde::Serialize`/`Deserialize` can be passed to/from JavaScript. To indicate
that a type `T` should be converted using `serde_v8`, write it as `serde<T>`,
like the `serde<u32>` above.

Use `#[js(func)]` to derive a Rust function that will call a corresponding
JavaScript function.

```rust
#[js(interface)]
impl Global {
    #[js(func)]
    fn btoa(&self, to_encode: serde<&str>) -> serde<String> {}
    // https://docs.deno.com/api/web/~/btoa
}
```

What if we want more than just the data? What if we would like to keep
JavaScript objects and values around so that we can use them later? Here comes
the fun part:

Thanks to the [`FromV8`][FromV8]/[`ToV8`][ToV8] traits, any Rust type derived
using this crate can also be passed from/to JavaScript (as can any type that
implements these traits). This is the default conversion mechanism if you don't
specify `serde<T>`.

Combining these attributes lets you statically declare JavaScript APIs of
arbitrary shapes. For example, here's how you declare the existence of
`console.log`:

```rust
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
```

Enough declaring! Let's finally call JavaScript:

```rust
// all APIs derived using this crate require a &mut JsRuntime
let rt: &mut JsRuntime = todo!("find a JsRuntime");

let global = Global::new(rt);
let console = global.console(rt)?;
let encoded = global.btoa(r#"{"alg":"HS256"}"#, rt)?;

console.log(&encoded, rt)?;
```

This is equivalent to the following JavaScript code:

```js
globalThis.console.log(globalThis.btoa(`{"alg":"HS256"}`));
// prints eyJhbGciOiJIUzI1NiJ9
```

(Provided, of course, that your `JsRuntime` actually has a `console` and a
`btoa` function).

To see the above in full, see the [_console_][example-console] example. To see
more of what this crate can derive, including [embedding ES modules][js-module],
[async functions][js-async], and [constructors][js-new], read on!

[globalThis]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/globalThis
[esm]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules
[newtype]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
[serde_v8]: https://crates.io/crates/serde_v8
[FromV8]: https://docs.rs/deno_core/0.338.0/deno_core/convert/trait.FromV8.html
[ToV8]: https://docs.rs/deno_core/0.338.0/deno_core/convert/trait.ToV8.html

## Examples

Run examples with `cargo run --package example-<name>`:

- [_calculator_][example-calculator] — loading modules and invoking constructors
- [_ts_][example-ts], [_ts-blank-space_][example-ts-blank-space] — workflows
  integrating NPM dependencies and a bundler

[example-console]: ./examples/console
[example-calculator]: ./examples/calculator
[example-ts]: ./examples/ts
[example-ts-blank-space]: ./examples/ts-blank-space

## Reference

## Discussions
