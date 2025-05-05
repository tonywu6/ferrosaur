# `#[js(func)]`

Use `#[js(func)]` for calling JavaScript functions.

<figure>

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
// Expressed in TypeScript:
interface Console {
  log(message: string): void;
}
declare let console: Console;
console.log("🦀 + 🦕");
```

</figure>

The generated function has the signature:

<div class="code-header">

#### fn \[function name](&self, \[args...,] \_rt: &mut [JsRuntime]) -> [anyhow::Result]\<...>

</div>

Argument types must implement either [`ToV8`] (the default) or [`Serialize`] (if written
as `serde<T>`). The return type must implement either [`FromV8`] or
[`DeserializeOwned`].

> [!NOTE]
>
> See [Specifying types](../typing.md) for more info on how you can specify types when
> using this crate.

Implicitly, the function name is the Rust function name
[converted to camelCase](heck::ToLowerCamelCase), but you can override this using the
[`name`](#option-name--) or [`Symbol`](#option-symbol) option.

<details class="toc" open>
  <summary>Sections</summary>

- [`async` functions](#async-functions)
- [`this` argument](#this-argument)
- [Spread arguments](#spread-arguments)
- [Option `name = "..."`](#option-name--)
- [Option `Symbol(...)`](#option-symbol)

</details>

## `async` functions

<figure>

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
// Expressed in TypeScript:
interface PromiseConstructor {
  resolve(value: number): Promise<number>;
}
declare let Promise: PromiseConstructor;
assert((await Promise.resolve(42)) === 42);
```

</figure>

The generated function will be an `async fn`. The returned [`Future`] will be ready once
the underlying JS value fulfills.

Internally, this calls [`JsRuntime::with_event_loop_promise`], which means you don't
need to drive the event loop separately.

## `this` argument

By default, the JS function will receive the object from which the function is accessed
(i.e. `&self`) as its `this` value. Expressed in TypeScript, the way your function is
invoked is roughly:

```ts
interface Foo {
  bar: () => void;
}
declare const foo: Foo;
const bar = foo.bar;
bar.call(foo);
```

Alternatively, you can explicitly declare the type of `this` using the second argument:

- [`this: undefined`](#this-undefined)
- [`this: [SomeType]`](#this-sometype)

### `this: undefined` <!-- omit from toc -->

<figure>

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Foo;
#[js(interface)]
impl Foo {
    #[js(func)]
    fn bar(&self, this: undefined) {}
}
```

```ts
// Expressed in TypeScript:
const bar = foo.bar;
bar.call(undefined);
```

</figure>

The derived Rust function will not have a `this` argument.

The JS function will receive a `this` value of `undefined` when called.

### `this: [SomeType]` <!-- omit from toc -->

<figure>

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Foo;
#[js(interface)]
impl Foo {
    #[js(func)]
    fn bar(&self, this: Baz) {}
}
#[js(value)]
struct Baz;
```

```ts
// Expressed in TypeScript:
const bar = foo.bar;
declare const baz: Baz;
bar.call(baz);
```

</figure>

The derived Rust function will have an explicit `this` argument, for which you will
supply a value at call time; the argument will be subject to the same
[type conversion](../typing.md) rules as other arguments.

## Spread arguments

To indicate an argument should be flattened using the spread syntax at call time, prefix
the argument name with `..` (2 dots):

<figure>

```rust
# use deno_core::v8;
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
# use fixture::items::global::Global;
#
# #[js(value)]
# struct Console;
#
#[js(interface)]
impl Console {
    #[js(func(name(log)))]
    pub fn log(&self, ..values: Vec<String>) {}
    //                ^
}
// let rt: &mut JsRuntime;
# let rt = &mut fixture::deno()?;
// let console: Console;
# let console: Console = fixture::eval_value("({ log: () => {} })", rt)?;
console.log(vec!["🦀".into(), "🦕".into()], rt)?;
#
# Ok::<_, anyhow::Error>(())
```

```ts
// Expressed in TypeScript:
interface Console {
  log: (...values: string[]) => void;
}
declare const console: Console;
console.log(...["🦀", "🦕"]);
```

</figure>

On the Rust side, a spread argument of type `A` must implement [`Iterator<Item = T>`],
where `T` must implement either [`ToV8`] (the default) or [`Serialize`] (if written as
`serde<T>`). When calling the function, pass the argument using normal syntax.

> [!NOTE]
>
> See [Specifying types](../typing.md) for more info on how you can specify types when
> using this crate.

> [!TIP]
>
> The syntax `..args: A` is abusing the [range pattern][range] syntax, which is
> [syntactically valid][patterns-intro] in function arguments.

## Option `name = "..."`

Use the specified string as key when accessing the function. This has the same usage as
[`js(prop(name))`](prop.md#option-name--).

You can also write `name(propertyKey)` if the key is identifier-like.

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

Use the specified [well-known Symbol][well-known-symbols] when accessing the function.
This has the same usage as [`js(prop(Symbol))`](prop.md#option-symbol).

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

<!-- prettier-ignore-start -->

[patterns-intro]: https://doc.rust-lang.org/reference/patterns.html#r-patterns.intro
[range]: https://doc.rust-lang.org/reference/patterns.html#range-patterns
[well-known-symbols]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Symbol#static_properties
[with_event_loop_promise]: deno_core::JsRuntime::with_event_loop_promise

<!-- prettier-ignore-end -->
