# `#[js(prop)]`

Generate a Rust function to access a JavaScript property.

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
#
# #[js(value)]
# struct Lorem;
#
#[js(interface)]
impl Lorem {
    #[js(prop)]
    fn ipsum(&self) -> serde<f64> {}
}
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
# let lorem: Lorem = fixture::eval_value("({ ipsum: 42 })", rt)?;
// let lorem: Lorem;
#
assert_eq!(lorem.ipsum(rt)?, 42.0);
#
# Ok(())
# }
```

```ts
// Expressed in TypeScript:
interface Lorem {
  ipsum: number;
}
declare let lorem: Lorem;
assert(lorem.ipsum === 42);
```

The generated function has the signature

<!-- deno-fmt-ignore-start -->
<span class="code-header">fn <span class="fn">\[name]</span>(&self, rt: &mut [JsRuntime]) -> [anyhow::Result]\<...></span>
<!-- deno-fmt-ignore-end -->

The return type indicates the expected type of the property, which should
implement either [`FromV8`][FromV8] (the default) or
[`DeserializeOwned`][DeserializeOwned] (if written as `serde<...>`). See [Type
conversions][TODO:] for more on how types are specified for this crate.

Implicitly, the property name is the Rust function name case-converted using
[`heck::ToLowerCamelCase`], but you can override this using the
[`name`](#option-jspropname--) or [`Symbol`](#option-jspropsymbol) option.

[anyhow::Result]: deno_core::anyhow::Result
[JsRuntime]: deno_core::JsRuntime
[FromV8]: deno_core::FromV8
[DeserializeOwned]: deno_core::serde::de::DeserializeOwned

<details class="toc" open>
  <summary>Table of contents</summary>

- [Option `name = "..."`](#option-name--)
- [Option `Symbol(...)`](#option-symbol)
- [Option `with_setter`](#option-with_setter)

</details>

## Option `name = "..."`

Use the specified string as key when accessing the JS property.

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Lorem;
#[js(interface)]
impl Lorem {
    #[js(prop(name = "some property"))]
    fn some_property(&self) -> serde<Option<u32>> {}
}
```

```ts
interface Lorem {
  "some property": number | null;
}
```

You can also write `name(propertyKey)` if the key is identifier-like.

## Option `Symbol(...)`

Use the specified [well-known Symbol][well-known-symbols] when accessing the JS
property. The Symbol should be in camel case (i.e. the same as in JS).

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Lorem;
#[js(interface)]
impl Lorem {
    #[js(prop(Symbol(toStringTag)))]
    fn to_string_tag(&self) -> serde<String> {}
}
```

```ts
interface Lorem {
  [Symbol.toStringTag]: string;
}
```

[well-known-symbols]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Symbol#static_properties

## Option `with_setter`

Generate a setter function in addition to a getter function. The function always
has the name `set_[getter_name]`.

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
#
# #[js(value)] struct Lorem;
#
#[js(interface)]
impl Lorem {
    #[js(prop(with_setter))]
    fn ipsum(&self) -> serde<f64> {}
}
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
# let lorem: Lorem = fixture::eval_value("({ ipsum: 42 })", rt)?;
// let lorem: Lorem;
#
lorem.set_ipsum(69.0, rt)?;
assert_eq!(lorem.ipsum(rt)?, 69.0);
#
# Ok(())
# }
```

```ts
interface Lorem {
  ipsum: number;
}
declare let lorem: Lorem;
lorem.ipsum = 69;
assert(lorem.ipsum === 69);
```

The generated function has the signature

<!-- deno-fmt-ignore-start -->
<span class="code-header">fn <span class="fn">set_\[getter_name]</span>(&self, value: ..., rt: &mut [JsRuntime]) -> [anyhow::Result]\<()></span>
<!-- deno-fmt-ignore-end -->

where `value` has the same type as the getter's declared return type.
