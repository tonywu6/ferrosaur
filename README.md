# `ferrosaur`

_So you use_ deno_core, _and you want to call JavaScript from Rust._

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
let module = Module::new(rt).await?;
let result = module.add(66.0, 3.0)?;
```

`ferrosaur` derives structs and functions from attributes (à la [wasm-bindgen])
that you can use with your [`JsRuntime`][JsRuntime].

[JsRuntime]: https://docs.rs/deno_core/0.338.0/deno_core/struct.JsRuntime.html
[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen#example
