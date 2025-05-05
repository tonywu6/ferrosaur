# ferrosaur

[![crates.io](https://img.shields.io/crates/v/ferrosaur?style=flat-square)](https://crates.io/crates/ferrosaur)
[![documentation](https://img.shields.io/github/actions/workflow/status/tonywu6/ferrosaur/docs.yml?event=release&style=flat-square&label=docs)](https://tonywu6.github.io/ferrosaur/)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/ferrosaur?style=flat-square)](https://github.com/tonywu6/ferrosaur/tree/main/LICENSE-APACHE.md)

_ferrosaur_ derives statically-typed Rust code — à la [wasm-bindgen] — for interfacing
between [`deno_core::JsRuntime`] and your JavaScript code.

<figure>

You have:

```javascript
// lib.js
export const add = (a, b) => a + b;
```

You write:

```rust
// lib.rs
#[js(module("lib.js"))]
struct Module;

#[js(interface)]
impl Module {
    #[js(func)]
    fn add(&self, a: serde<f64>, b: serde<f64>) -> serde<f64> {}
}
```

You get:

```rust
// let rt: &mut JsRuntime;
let module = Module::main_module_init(rt).await?;
let result = module.add(66.0, 3.0)?;
```

</figure>

## License

This project is released under the
[Apache 2.0 License](https://github.com/tonywu6/ferrosaur/tree/main/LICENSE-APACHE.md)
and the [MIT License](https://github.com/tonywu6/ferrosaur/tree/main/LICENSE-MIT.md).

<!-- prettier-ignore-start -->

[`deno_core::JsRuntime`]: https://docs.rs/deno_core/latest/deno_core/struct.JsRuntime.html
[wasm-bindgen]:           https://github.com/rustwasm/wasm-bindgen#example

<!-- prettier-ignore-end -->
