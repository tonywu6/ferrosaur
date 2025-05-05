# Reference

## Entrypoints

- [`js(module)`](module.md), embed and load ES modules.
- [`js(global_this)`](global-this.md), access the `globalThis` object.

## Newtypes

- [`js(value)`](value.md), give arbitrary JavaScript values a Rust type.

## Interfaces

- [`js(interface)`](interface.md), declare object [properties](interface/prop.md),
  [functions](interface/func.md), and [constructors](interface/new.md).
- [`js(callable)`](callable.md), store JavaScript functions as values.
- [`js(iterator)`](iterator.md), bridge between JavaScript and Rust iterators.

<!-- prettier-ignore-start -->

[JsRuntime]:    https://docs.rs/deno_core/0.338.0/deno_core/struct.JsRuntime.html
[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen#example

<!-- prettier-ignore-end -->
