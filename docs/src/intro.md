# Introduction

_ferrosaur_ lets you declare statically-typed bindings, à la [wasm-bindgen], that you
can then use with [`deno_core::JsRuntime`][JsRuntime].

## Entrypoints

- [`js(module)`](reference/module.md), embed and load ES modules.
- [`js(global_this)`](reference/global-this.md), access the `globalThis` object.

## Newtypes

- [`js(value)`](reference/value.md), give arbitrary JavaScript values a Rust type.

## Interfaces

- [`js(interface)`](reference/interface.md), declare object
  [properties](reference/interface/prop.md), [functions](reference/interface/func.md),
  and [constructors](reference/interface/new.md).
- [`js(callable)`](reference/callable.md), store JavaScript functions as values.
- [`js(iterator)`](reference/iterator.md), bridge between JavaScript and Rust iterators.

<!-- prettier-ignore-start -->

[JsRuntime]: https://docs.rs/deno_core/0.338.0/deno_core/struct.JsRuntime.html
[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen#example

<!-- prettier-ignore-end -->
