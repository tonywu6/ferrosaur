# `#[js(new)]`

Generate a Rust function to invoke a JavaScript constructor.

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
// struct Date; impl Date { ... }
#
# Ok(())
# }
```

```ts
let date = new Date(0);
assert(date.toISOString() === "1970-01-01T00:00:00.000Z");
```

The generated function has the signature

<!-- deno-fmt-ignore-start -->
<span class="code-header">fn <span class="fn">\[name]</span>(&self, \[args...,] _rt: &mut [JsRuntime]) -> [anyhow::Result]\<...></span>
<!-- deno-fmt-ignore-end -->

This accepts the same argument and return type format as [`js(func)`](func.md).
Note that constructors cannot be async.

Implicitly, the class name is the name of the return type (with case preserved).
If the return type name cannot be used, such as if it is not a simple
identifier, or if you would like to override it, you can use the
[`class`](#jsnewclass) option.

[JsRuntime]: deno_core::JsRuntime

<details class="toc" open>
  <summary>Table of contents</summary>

- [Placement](#placement)
- [Option `class(...)`](#option-class)

</details>

## Placement

Note that the return type indicates _the JavaScript type that will be
constructed_, whereas `&self` represents _the JavaScript object that has access
to the constructor_ (such as a module or `globalThis`).

In other words, the following usage is almost never correct:

```rust,compile_fail
# use ferrosaur::js;
// 🔴 these are almost never what you want
#[js(interface)]
struct Rectangle {
    #[js(new)]
    fn new() -> Self {}
    // or
    #[js(new)]
    fn new(&self) -> Self {}
}
```

## Option `class(...)`

Use the specified key when accessing the constructor. Has the same usage as
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
