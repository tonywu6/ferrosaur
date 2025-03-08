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
let module = Module::main_module_init(rt).await?;
let result = module.add(66.0, 3.0)?;
```

_ferrosaur_ derives structs and functions from attributes (à la [wasm-bindgen])
that you can use with your favorite [`JsRuntime`][JsRuntime].

[JsRuntime]: https://docs.rs/deno_core/0.338.0/deno_core/struct.JsRuntime.html
[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen#example

## Examples

- [_console_][example-console] — intro using console.log, start from here!
- [_calculator_][example-calculator] — loading ES modules
- [_ts_][example-ts], [_ts-blank-space_][example-ts-blank-space] — workflows
  integrating NPM dependencies and a bundler

You can run examples with `cargo run --package example-<name>`.

[example-console]: ./examples/console/src/main.rs
[example-calculator]: ./examples/calculator
[example-ts]: ./examples/ts
[example-ts-blank-space]: ./examples/ts-blank-space
