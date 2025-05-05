# `#[js(prop)]`

Use `#[js(prop)]` for access to JavaScript properties.

<figure>

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
#
# #[js(value)]
# struct Foo;
#
#[js(interface)]
impl Foo {
    #[js(prop)]
    fn bar(&self) -> serde<f64> {}
}
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
# let foo: Foo = fixture::eval_value("({ bar: 42 })", rt)?;
// let foo: Foo;
#
assert_eq!(foo.bar(rt)?, 42.0);
#
# Ok(())
# }
```

```ts
// Expressed in TypeScript:
interface Foo {
  bar: number;
}
declare let foo: Foo;
assert(foo.bar === 42);
```

</figure>

The generated function has the signature:

<div class="code-header">

#### fn \[property name](&self, rt: &mut [JsRuntime]) -> [anyhow::Result]\<...>

</div>

The return type indicates the expected type of the property, which must implement either
[`FromV8`] (the default) or [`DeserializeOwned`] (if written as `serde<...>`).

> [!NOTE]
>
> See [Specifying types](../typing.md) for more info on how you can specify types when
> using this crate.

Implicitly, the property name is the Rust function name
[converted to camelCase](heck::ToLowerCamelCase), but you can override this using the
[`name`](#option-name--) or [`Symbol`](#option-symbol) option.

<details class="toc" open>
  <summary>Sections</summary>

- [Option `name = "..."`](#option-name--)
- [Option `Symbol(...)`](#option-symbol)
- [Option `with_setter`](#option-with_setter)

</details>

## Option `name = "..."`

Use the specified string as key when accessing the JS property.

You can also write `name(propertyKey)` if the key is identifier-like.

<figure>

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Foo;
#[js(interface)]
impl Foo {
    #[js(prop(name = "some bar"))]
    fn some_bar(&self) -> serde<Option<u32>> {}
}
```

```ts
// Expressed in TypeScript:
interface Foo {
  "some bar": number | null;
}
```

</figure>

## Option `Symbol(...)`

Use the specified [well-known Symbol][well-known-symbols] when accessing the JS
property. The Symbol should be in camel case (i.e. the same as in JS).

<figure>

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Foo;
#[js(interface)]
impl Foo {
    #[js(prop(Symbol(toStringTag)))]
    fn to_string_tag(&self) -> serde<String> {}
}
```

```ts
// Expressed in TypeScript:
interface Foo {
  [Symbol.toStringTag]: string;
}
```

</figure>

## Option `with_setter`

Generate a setter function in addition to a getter function.

<figure>

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
#
# #[js(value)]
# struct Foo;
#
#[js(interface)]
impl Foo {
    #[js(prop(with_setter))]
    fn bar(&self) -> serde<f64> {}
}
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
# let foo: Foo = fixture::eval_value("({ bar: 41 })", rt)?;
// let foo: Foo;
#
assert_eq!(foo.bar(rt)?, 41.0);
foo.set_bar(42.0, rt)?;
assert_eq!(foo.bar(rt)?, 42.0);
#
# Ok(())
# }
```

```ts
// Expressed in TypeScript:
interface Foo {
  bar: number;
}
declare let foo: Foo;
assert(foo.bar === 41);
foo.bar = 42;
assert(foo.bar === 42);
```

</figure>

The generated function has the signature:

<div class="code-header">

#### fn set\_\[getter name](&self, value: ..., rt: &mut [JsRuntime]) -> [anyhow::Result]\<()> <!-- omit from toc -->

</div>

where `value` has the same type as the getter's declared return type.

<!-- prettier-ignore-start -->

[well-known-symbols]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Symbol#static_properties

<!-- prettier-ignore-end -->
