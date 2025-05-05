# `#[js(new)]`

Use `#[js(func)]` for invoking JavaScript constructors.

<figure>

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
#
# #[js(global_this)]
# struct Global;
#
#[js(interface)]
impl Global {
    #[js(new)]
    fn date(&self, timestamp: serde<f64>) -> Date {}
}
#
# #[js(value)]
# struct Date;
# #[js(interface)]
# impl Date {
#     #[js(func(name(toISOString)))]
#     fn to_iso_string(&self) -> serde<String> {}
# }
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
# let global = Global::new(rt);
// let global: Global;
#
let date = global.date(0.0, rt)?;
assert_eq!(date.to_iso_string(rt)?, "1970-01-01T00:00:00.000Z");
// struct Date;
// impl Date { ... }
#
# Ok(())
# }
```

```ts
// Expressed in TypeScript:
let date = new Date(0);
assert(date.toISOString() === "1970-01-01T00:00:00.000Z");
```

</figure>

The generated function has the signature:

<div class="code-header">

#### fn \[name](&self, \[args...,] \_rt: &mut [JsRuntime]) -> [anyhow::Result]\<...>

</div>

`js(new)` accepts the same function signature format as [`js(func)`](func.md), except
constructors cannot be `async`.

Implicitly, the class name is the name of the return type (with case preserved). If the
return type name cannot be used, such as if it is not a simple identifier, or if you
would like to override it, you can use the [`class`](#option-class) option.

<details class="toc" open>
  <summary>Sections</summary>

- [Note on return type](#note-on-return-type)
- [Option `class(...)`](#option-class)

</details>

## Note on return type

Note that the return type of the method is _the JavaScript type that will be
constructed_, whereas `Self` represents _the JavaScript object from which the
constructor is accessed_ (such as a module or `globalThis`).

In other words, the following usage is almost never what you want:

<figure>

```rust,compile_fail
# use ferrosaur::js;
// 🔴 these are almost never what you want
#[js(interface)]
impl Rectangle {
    #[js(new)]
    fn new() -> Self {}
    // or
    #[js(new)]
    fn new(&self) -> Self {}
}
```

</figure>

Instead, you likely want to write:

<figure>

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Shapes;
#[js(interface)]
impl Shapes {
    #[js(new)]
    fn rectangle(&self) -> Rectangle {}
}
#[js(value)]
struct Rectangle;
```

```ts
// Expressed in TypeScript:
declare const shapes: Shapes;
const Rectangle = shapes.Rectangle;
new Rectangle();
```

</figure>

## Option `class(...)`

Use the specified string as key when accessing the constructor, instead of using the
name of the return type. This has the same usage as
[`js(prop(name))`](prop.md#option-name--).

```rust
# use ferrosaur::js;
# #[js(value)]
# struct Window;
#[js(interface)]
impl Window {
    #[js(new(class(XMLHttpRequest)))]
    fn xml_http_request(&self) -> XmlHttpRequest {}
}
# #[js(value)]
# struct XmlHttpRequest;
```
