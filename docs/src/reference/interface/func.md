# `#[js(func)]`

Generate a Rust function to call a JavaScript function.

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
#
# #[js(value)]
# struct Console;
#
#[js(interface)]
impl Console {
    #[js(func)]
    fn log(&self, message: serde<&str>) {}
}
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
# let console: Console = fixture::eval_value("({ log: () => {} })", rt)?;
// let console: Console;
#
console.log("🦀 + 🦕", rt)?;
#
# Ok(())
# }
```

```ts
interface Console {
  log(message: string): void;
}
declare let console: Console;
console.log("🦀 + 🦕");
```

The generated function has the signature

<!-- deno-fmt-ignore-start -->
<span class="code-header">fn <span class="fn">\[name]</span>(&self, \[args...,] _rt: &mut [JsRuntime]) -> [anyhow::Result]\<...></span>
<!-- deno-fmt-ignore-end -->

Argument types should implement either [`ToV8`][ToV8] (the default) or
[`Serialize`][Serialize] (if written as `serde<T>`); the return type should
implement either [`FromV8`][FromV8] or [`DeserializeOwned`][DeserializeOwned].

Implicitly, the function name is the Rust function name case-converted using
[`heck::ToLowerCamelCase`], but you can override this using the
[`name`](#option-name--) or [`Symbol`](#option-symbol) option.

`js(func)` supports [async functions](#async-functions) and [variadic
arguments][TODO:].

[JsRuntime]: deno_core::JsRuntime
[ToV8]: deno_core::ToV8
[Serialize]: deno_core::serde::ser::Serialize
[FromV8]: deno_core::FromV8
[DeserializeOwned]: deno_core::serde::de::DeserializeOwned

<details class="toc" open>
  <summary>Table of contents</summary>

- [`async` functions](#async-functions)
- [`this` argument](#this-argument)
- [Option `name = "..."`](#option-name--)
- [Option `Symbol(...)`](#option-symbol)

</details>

## `async` functions

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
# use fixture::items::global::Global;
#
# #[js(interface)]
# impl Global {
#     #[js(prop(name(Promise)))]
#     fn promise_constructor(&self) -> PromiseConstructor {}
# }
#
# #[js(value)]
# struct PromiseConstructor;
#
#[js(interface)]
impl PromiseConstructor {
    #[js(func)]
    async fn resolve(&self, value: serde<u64>) -> serde<u64> {}
}
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
# let global = Global::new(rt);
# #[allow(non_snake_case)]
# let Promise = global.promise_constructor(rt)?;
// let Promise: PromiseConstructor;
#
assert_eq!(Promise.resolve(42, rt).await?, 42);
#
# Ok(())
# }
```

```ts
interface PromiseConstructor {
  resolve(value: number): Promise<number>;
}
declare let Promise: PromiseConstructor;
assert(await Promise.resolve(42) === 42);
```

The generated function will be an `async fn` instead of a normal `fn`. The
returned [`Future`][Future] will be ready once the underlying JS value fulfills.

This uses [`JsRuntime::with_event_loop_promise`][with_event_loop_promise] under
the hood, which drives the event loop for you.

[Future]: std::future::Future
[with_event_loop_promise]: deno_core::JsRuntime::with_event_loop_promise

## `this` argument

By default, the JS function has a `this` value of the object from which the
function is accessed (i.e. `&self`).

Alternatively, you can explicitly declare the type of `this` using the first
argument:

### `this: undefined` <!-- omit from toc -->

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Object;
#[js(interface)]
impl Object {
    #[js(func)]
    fn function(&self, this: undefined) {}
}
```

`this` will be `undefined`; the generated function will not have a `this`
argument.

### `this: [SomeType]` <!-- omit from toc -->

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Object;
#[js(interface)]
impl Object {
    #[js(func)]
    fn function(&self, this: SomeType) {}
}
# #[js(value)]
# struct SomeType;
// struct SomeType;
```

This allows you to explicitly pass a value as `this` when calling the function,
which will be subject to the same [type conversion][TODO:] rules as normal
arguments.

## Option `name = "..."`

Overrides the property key when accessing the function. Has the same usage as
[`js(prop(name))`](prop.md#option-name--).

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Date;
#[js(interface)]
impl Date {
    #[js(func(name(toISOString)))]
    fn to_iso_string(&self) -> serde<String> {}
}
```

## Option `Symbol(...)`

Overrides the property key when accessing the function. Has the same usage as
[`js(prop(Symbol))`](prop.md#option-symbol)</span>.

```rust
# use ferrosaur::js;
# use deno_core::serde_json;
# #[js(value)]
# struct Date;
#[js(interface)]
impl Date {
    #[js(func(Symbol(toPrimitive)))]
    fn to_primitive(&self, hint: serde<&str>) -> serde<serde_json::Value> {}
}
```
