# ferrosaur

<div class="badges">

[![crates.io](https://img.shields.io/crates/v/ferrosaur?style=flat-square)](https://crates.io/crates/ferrosaur)
[![documentation](https://img.shields.io/github/actions/workflow/status/tonywu6/ferrosaur/docs.yml?event=release&style=flat-square&label=docs)](https://tonywu6.github.io/ferrosaur/)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/ferrosaur?style=flat-square)](https://github.com/tonywu6/ferrosaur/tree/main/LICENSE-APACHE.md)

</div>

_ferrosaur_ derives statically-typed Rust code — à la [wasm-bindgen] — for
[`deno_core::JsRuntime`] to interface with your JavaScript code.

<figure>

If you have:

```js
// lib.js
export const slowFib = (n) =>
  n === 0 ? 0 : n === 1 ? 1 : slowFib(n - 1) + slowFib(n - 2);
```

... and you write:

```rust
// lib.rs
use ferrosaur::js;

#[js(module("lib.js"))]
struct Math;

#[js(interface)]
impl Math {
    #[js(func)]
    fn slow_fib(&self, n: serde<usize>) -> serde<usize> {}
}
```

... then you get:

```rust
// let rt: &mut JsRuntime;
let lib = Math::main_module_init(rt).await?;
let fib = lib.slow_fib(42, rt)?;
assert_eq!(fib, 267914296);
```

</figure>

> [!TIP]
>
> This is like the inverse of [`deno_core::op2`]:
>
> - `#[op2]` gives JavaScript programs easy access to your Rust implementation.
> - _ferrosaur_ gives your Rust program easy access to JavaScript implementations.

## License

This project is released under the
[Apache 2.0 License](https://github.com/tonywu6/ferrosaur/tree/main/LICENSE-APACHE.md)
and the [MIT License](https://github.com/tonywu6/ferrosaur/tree/main/LICENSE-MIT.md).

<!-- prettier-ignore-start -->

[`deno_core::JsRuntime`]: https://docs.rs/deno_core/latest/deno_core/struct.JsRuntime.html
[`deno_core::op2`]:       https://docs.rs/deno_ops/latest/deno_ops/attr.op2.html
[wasm-bindgen]:           https://github.com/rustwasm/wasm-bindgen#example

<!-- prettier-ignore-end -->
