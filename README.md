# ferrosaur

[![crates.io](https://img.shields.io/crates/v/ferrosaur?style=flat-square)](https://crates.io/crates/ferrosaur)
[![documentation](https://img.shields.io/github/actions/workflow/status/tonywu6/ferrosaur/docs.yml?event=release&style=flat-square&label=docs)](https://tonywu6.github.io/ferrosaur/)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/ferrosaur?style=flat-square)](https://github.com/tonywu6/ferrosaur/tree/main/LICENSE-APACHE.md)

<em/>So you use</em> [`deno_core`], <em>and you want to call JavaScript from Rust.</em>

```javascript
export const add = (a, b) => a + b;
```

```rust
use ferrosaur::js;

#[js(module("lib.js"))]
struct Module;

#[js(interface)]
impl Module {
    #[js(func)]
    fn add(&self, a: serde<f64>, b: serde<f64>) -> serde<f64> {}
}
```

_ferrosaur_ derives types and implementations, à la [wasm-bindgen], that you can use
with your favorite [`JsRuntime`].

## Documentation

[Read the book!](https://tonywu6.github.io/ferrosaur/)

You may be looking for:

<!-- prettier-ignore-start -->

- [Examples](https://tonywu6.github.io/ferrosaur/examples)

  - [_console.log_](https://tonywu6.github.io/ferrosaur/examples/console)
    | [_calculator_](https://tonywu6.github.io/ferrosaur/examples/calculator)
    | [_ts_](https://tonywu6.github.io/ferrosaur/examples/ts)
    | [_ts-blank-space_](https://tonywu6.github.io/ferrosaur/examples/ts-blank-space)

- [Reference](https://tonywu6.github.io/ferrosaur/reference)

  - [`js(global_this)`](https://tonywu6.github.io/ferrosaur/reference/global-this)
    | [`js(module)`](https://tonywu6.github.io/ferrosaur/reference/module)
    | [`js(value)`](https://tonywu6.github.io/ferrosaur/reference/value)
    | [`js(interface)`](https://tonywu6.github.io/ferrosaur/reference/interface)
    | [`js(prop)`](https://tonywu6.github.io/ferrosaur/reference/interface/prop)
    | [`js(func)`](https://tonywu6.github.io/ferrosaur/reference/interface/func)
    | [`js(new)`](https://tonywu6.github.io/ferrosaur/reference/interface/new)

<!-- prettier-ignore-end -->

## License

This project is released under the
[Apache 2.0 License](https://github.com/tonywu6/ferrosaur/tree/main/LICENSE-APACHE.md)
and the [MIT License](https://github.com/tonywu6/ferrosaur/tree/main/LICENSE-MIT.md).

<!-- prettier-ignore-start -->

[`deno_core`]:  https://docs.rs/deno_core
[`JsRuntime`]:  https://docs.rs/deno_core/latest/deno_core/struct.JsRuntime.html
[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen#example

<!-- prettier-ignore-end -->
