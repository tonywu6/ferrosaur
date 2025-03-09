_ferrosaur_ derives structs and functions from attributes (à la [wasm-bindgen])
that you can use with your favorite [`JsRuntime`][JsRuntime].

This is the reference. For an intro, see the [_console_][console] example. For
more information, see the GitHub [README].

[JsRuntime]: deno_core::JsRuntime
[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen#example
[console]: https://github.com/tonywu6/ferrosaur/blob/main/examples/console/src/main.rs
[README]: https://github.com/tonywu6/ferrosaur/blob/main/README.md

<details>
  <summary><strong>Table of Contents</strong></summary>

- [Reference](#reference)
  - [`#[js(module)]`](#jsmodule)
    - [Trait implementations](#trait-implementations)
    - [Option: #\[js(module(..., `fast`))\]](#option-jsmodule-fast)
    - [Option: #\[js(module(..., `fast(unsafe_debug)`))\]](#option-jsmodule-fastunsafe_debug)
    - [Option: #\[js(module(..., `url(...)`))\]](#option-jsmodule-url)
  - [`#[js(value)]`](#jsvalue)
    - [Trait Implementations](#trait-implementations-1)
    - [Option: #\[js(value(`of_type(T)`))\]](#option-jsvalueof_typet)
  - [`#[js(interface)]`](#jsinterface)
    - [`#[js(prop)]`](#jsprop)
      - [Option: #\[js(prop(`name = "..."`))\]](#option-jspropname--)
      - [Option: #\[js(prop(`Symbol(...)`))\]](#option-jspropsymbol)
      - [Option: #\[js(prop(`with_setter`))\]](#option-jspropwith_setter)
    - [`#[js(func)]`](#jsfunc)
      - [`async` functions](#async-functions)
      - [`this` argument](#this-argument)
      - [Option: #\[js(func(`name = "..."`))\]](#option-jsfuncname--)
      - [Option: #\[js(func(`Symbol(...)`))\]](#option-jsfuncsymbol)
    - [`#[js(new)]`](#jsnew)
      - [Placement](#placement)
      - [Option: #\[js(new(`class(...)`))\]](#option-jsnewclass)
    - [`#[js(get_index)]`, `#[js(set_index)]`](#jsget_index-jsset_index)
  - [`#[js(function)]`](#jsfunction)
  - [`#[js(iterator)]`](#jsiterator)
  - [Type conversions](#type-conversions)
    - [`Self` type](#self-type)
    - [Variadic arguments](#variadic-arguments)
- [Guides](#guides)
  - [Having modules import from each other](#having-modules-import-from-each-other)
  - [Classes with static properties](#classes-with-static-properties)
  - [Emulating inheritance](#emulating-inheritance)

</details>

# Reference

---

## `#[js(module)]`

Embed an [ECMAScript module][esm] in the program.

```rust
# use ferrosaur::js;
# #[path = "../tests/util/mod.rs"]
# mod util;
#[js(module("../examples/js/mod.js"))]
pub struct Module;
```

The path is relative to the current file (it has the same usage as the
[`include_str!`] macro).

Call the associated `main_module_init` function to initialize it as a
[main module][load_main_es_module_from_code], or call the associated
`side_module_init` function to initialize it as a
[side module][load_side_es_module_from_code]:

```rust
# #[path = "../tests/util/mod.rs"]
# mod util;
# use util::compile::modules::{Main, Iter as Side};
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut util::deno()?;
// let rt: &mut JsRuntime;
let main = Main::main_module_init(rt).await?;
let side = Side::side_module_init(rt).await?;
# Ok(())
# }
```

[esm]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules
[load_main_es_module_from_code]: deno_core::JsRuntime::load_main_es_module_from_code
[load_side_es_module_from_code]: deno_core::JsRuntime::load_side_es_module_from_code

### Options <!-- omit from toc -->

- [`fast`](#jsmodule-fast)
- [`fast(unsafe_debug)`](#jsmodule-fastunsafe_debug)
- [`url(...)`](#jsmodule-url)

### Trait implementations

<!-- deno-fmt-ignore-start -->

- <span class="code-header">impl [AsRef]<[v8::Global]<[v8::Object]>> for Module</span>
- <span class="code-header">impl<\'a> [ToV8]<\'a> for Module</span>
- <span class="code-header">impl<\'a> [ToV8]<\'a> for &\'_ Module</span>

<!-- deno-fmt-ignore-end -->

### Option: #[js(module(..., `fast`))]

Without the `fast` option, JavaScript source code is embedded using
[`include_str!`].

With the `fast` option, JavaScript source code is embedded using
[`deno_core::ascii_str_include!`][deno_core::ascii_str_include] instead.

**The JS source file must be in 7-bit ASCII.** It is a compile error if this is
not true.

See [`FastStaticString`][FastStaticString] and
[`ascii_str_include!`][deno_core::ascii_str_include] for more info.

[FastStaticString]: deno_core::FastStaticString

### Option: #[js(module(..., `fast(unsafe_debug)`))]

Like `fast`, but for debug builds, unsafely embeds JS code as
`FastStaticString`s [without checking][unchecked] it is in 7-bit ASCII at
compile time. For release builds, behaves the same as `fast`. This uses the
`#[cfg(debug_assertions)]` condition.

It is undefined behavior if the file is not actually ASCII.

This could be useful if the source file you are trying to embed is very large,
in which case the compilation could take a very long time while const eval tries
to assert the entire file to be ASCII (this macro will
`#[allow(long_running_const_eval)]` but you will still see a warning).

See [`unsafe_include_fast_string`][crate::unsafe_include_fast_string!] which is
the macro under the hood.

[unchecked]: deno_core::v8::String::create_external_onebyte_const_unchecked

### Option: #[js(module(..., `url(...)`))]

Controls the [`import.meta.url`][import-meta-url] within the module.

Choices are:

- `url(preserve)` (default)

  - URL will be `file:///` + relative path to [`CARGO_MANIFEST_DIR`][cargo-env].

    For example, if the JS file is at `src/js/index.js`, then the URL is
    `"file:///src/js/index.js"`

- `url(cwd)`

  - URL will be `file:///` + [`std::env::current_dir()`] _at runtime_ + a name
    generated from the file's relative path

    For example, if the JS file is at `src/js/index.js`, and the program's
    runtime working directory is `/path/to/cwd`, then the URL is
    `"file:///path/to/cwd/-src-js-index.js"`

- `url("...")`

  - Use a custom URL. The string must be parseable by
    [`url::Url`][deno_core::url::Url]. It is a runtime error if the URL is not
    valid.

    For example, `url("npm:lodash")` causes `import.meta.url` to be
    `"npm:lodash"`.

See the [Guides](#having-modules-import-from-each-other) section for the uses
for these options.

[import-meta-url]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta#url
[cargo-env]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates

---

## `#[js(value)]`

Generate a [newtype] struct to hold a reference to an arbitrary JavaScript
value.

```rust
# use ferrosaur::js;
#[js(value)]
struct Lorem;
```

Use this to generate custom Rust types for which you can further derive
implementations: see [`#[js(interface)]`](#jsinterface).

You cannot initialize a `#[js(value)]` struct directly. Instead, there are
several ways:

- Return it from a [`#[js(prop)]`](#jsprop) or [`#[js(func)]`](#jsfunc), etc.
- Use one of the conversion traits, see below.

### Options <!-- omit from toc -->

- [`of_type(T)`](#jsvalueof_typet)

### Trait Implementations

<!-- deno-fmt-ignore-start -->

- <span class="code-header">impl [From]<[v8::Global]\<_T_>> for Lorem</span>
- <span class="code-header">impl<\'a> [FromV8]<\'a> for Lorem</span>
- <span class="code-header">impl [AsRef]<[v8::Global]\<_T_>> for Lorem</span>
- <span class="code-header">impl<\'a> [ToV8]<\'a> for Lorem</span>
- <span class="code-header">impl<\'a> [ToV8]<\'a> for &\'_ Lorem</span>
- <span class="code-header">impl [From]\<Lorem> for [v8::Global]\<_T_></span>

<!-- deno-fmt-ignore-end -->

... where `<T>` is the one of the `v8::*` data types. By default, this is
[`v8::Value`][v8::Value], but you can control it using the
[`of_type`](#jsvalueof_typet) option.

[FromV8]: deno_core::FromV8

### Option: #[js(value(`of_type(T)`))]

By default, `#[js(value)]` generates a struct that is:

```rust
use deno_core::v8;
struct Lorem(v8::Global<v8::Value>);
```

By using the `of_type` option, you can use some other V8 data types for the
inner type. For example:

```rust
# use ferrosaur::js;
#[js(value(of_type(v8::Promise)))]
struct Response;
// struct Response(v8::Global<v8::Promise>);
```

Note that you don't specify the `v8::Global<...>` part.

It should make sense for the data type `T` to be placed in a `v8::Global`. In
particular, this means `v8::Local<v8::Value>` implements
`TryInto<v8::Local<T>>`. See [`v8::Local`][v8::Local].

This could be useful if you want to have simple runtime type checking for your
types. For example, given the `Response` type above, if a JS function is
supposed to return a `Response`, i.e. a `Promise`, but it returns `undefined`,
then the corresponding Rust function returns `Err(...)` instead of
`Ok(Response)`.

Note that this is "type checking" only in so far as [`v8`][v8] can try-convert
between different V8 types; this is not TypeScript-style structural typing.

See [Type conversions](#type-conversions) for more on how types are specified
for this crate.

[v8]: deno_core::v8
[v8::Value]: deno_core::v8::Value
[v8::Local]: deno_core::v8::Local#trait-implementations

## `#[js(interface)]`

Declare JavaScript "interfaces":

```rust
# use ferrosaur::js;
# #[path = "../tests/util/mod.rs"]
# mod util;
# use util::compile::global::Global;
#[js(interface)]
impl Global {
    // ...
}
```

You can declare:

- [`#[js(prop)]`](#jsprop) — property accessors
- [`#[js(func)]`](#jsfunc) — functions
- [`#[js(new)]`](#jsnew) — constructors
- [`#[js(get_index)]`, `#[js(set_index)]`](#jsget_index-jsset_index) — indexing
  getters and setters

You can use `#[js(interface)]` to generate implementations for any Rust types
generated using this crate, see [`Self` type](#self-type) for more info. You can
even use `#[js(interface)]` on traits, see
[Emulating inheritance](#emulating-inheritance).

### `#[js(prop)]`

Generate a Rust function to access a JavaScript property.

```rust
# use ferrosaur::js;
# #[path = "../tests/util/mod.rs"]
# mod util;
# #[js(value)]
# struct Lorem;
#
#[js(interface)]
impl Lorem {
    #[js(prop)]
    fn ipsum(&self) -> serde<f64> {}
}
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut util::deno()?;
// let rt: &mut JsRuntime;
# let lorem: Lorem = util::eval_value("({ ipsum: 42 })", rt)?;
// let lorem: Lorem;
assert_eq!(lorem.ipsum(rt)?, 42.0);
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
> <span class="code-header">fn <span class="fn">\[name]</span>(&self, rt: &mut [JsRuntime]) -> [anyhow::Result]\<...></span>
<!-- deno-fmt-ignore-end -->

The return type indicates the expected type of the property, which should
implement either [`FromV8`][FromV8] (the default) or
[`DeserializeOwned`][DeserializeOwned] (if written as `serde<...>`). See
[Type conversions](#type-conversions) for more on how types are specified for
this crate.

Implicitly, the property name is the Rust function name case-converted using
[`heck::ToLowerCamelCase`], but you can override this using the
[`name`](#jspropname--) or [`Symbol`](#jspropsymbol) option.

[DeserializeOwned]: deno_core::serde::de::DeserializeOwned
[anyhow::Result]: deno_core::anyhow::Result

#### Options <!-- omit from toc -->

- [`name = "..."`](#jspropname--)
- [`Symbol(...)`](#jspropsymbol)
- [`with_setter`](#jspropwith_setter)

#### Option: #[js(prop(`name = "..."`))]

Use the specified key when accessing the JS property.

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

#### Option: #[js(prop(`Symbol(...)`))]

Use the specified [well-known Symbol][well-known-symbols] when accessing the JS
property. The Symbol should be in camel case (should be the same as in JS).

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

#### Option: #[js(prop(`with_setter`))]

Generate a setter function in addition to a getter function. The function always
has the name `set_[getter_name]`.

```rust
# use ferrosaur::js;
# #[path = "../tests/util/mod.rs"]
# mod util;
# #[js(value)] struct Lorem;
#
#[js(interface)]
impl Lorem {
    #[js(prop(with_setter))]
    fn ipsum(&self) -> serde<f64> {}
}
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut util::deno()?;
// let rt: &mut JsRuntime;
# let lorem: Lorem = util::eval_value("({ ipsum: 42 })", rt)?;
// let lorem: Lorem;
lorem.set_ipsum(69.0, rt)?;
assert_eq!(lorem.ipsum(rt)?, 69.0);
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
> <span class="code-header">fn <span class="fn">set_\[getter_name]</span>(&self, value: ..., rt: &mut [JsRuntime]) -> [anyhow::Result]\<()></span>
<!-- deno-fmt-ignore-end -->

... where `value` has the same type as the getter's declared return type.

### `#[js(func)]`

Generate a Rust function to call a JavaScript function.

```rust
# use ferrosaur::js;
# #[path = "../tests/util/mod.rs"]
# mod util;
# #[js(value)]
# struct Console;
#
#[js(interface)]
impl Console {
    #[js(func)]
    fn log(&self, message: serde<&str>) {}
}
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut util::deno()?;
// let rt: &mut JsRuntime;
# let console: Console = util::eval_value("({ log: () => {} })", rt)?;
// let console: Console;
console.log("🦀", rt)?;
# Ok(())
# }
```

```ts
interface Console {
  log(message: string): void;
}
declare let console: Console;
console.log("🦀");
```

The generated function has the signature

<!-- deno-fmt-ignore-start -->
> <span class="code-header">fn <span class="fn">\[name]</span>(&self, \[args...,] _rt: &mut [JsRuntime]) -> [anyhow::Result]\<...></span>
<!-- deno-fmt-ignore-end -->

Argument types should implement either [`ToV8`][ToV8] (the default) or
[`Serialize`][Serialize] (if written as `serde<T>`); the return type should
implement either [`FromV8`][FromV8] or [`DeserializeOwned`][DeserializeOwned].

Implicitly, the function name is the Rust function name case-converted using
[`heck::ToLowerCamelCase`], but you can override this using the
[`name`](#jsfuncname--) or [`Symbol`](#jsfuncsymbol) option.

`#[js(func)]` supports [async functions](#async-functions) and
[variadic arguments](#variadic-arguments).

[Serialize]: deno_core::serde::ser::Serialize

#### Options <!-- omit from toc -->

- [`name = "..."`](#jsfuncname--)
- [`Symbol(...)`](#jsfuncsymbol)

#### `async` functions

```rust
# use ferrosaur::js;
#
# #[path = "../tests/util/mod.rs"]
# mod util;
# use util::compile::global::Global;
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
# let rt = &mut util::deno()?;
// let rt: &mut JsRuntime;
# let global = Global::new(rt);
# #[allow(non_snake_case)]
# let Promise = global.promise_constructor(rt)?;
// let Promise: PromiseConstructor;
assert_eq!(Promise.resolve(42, rt).await?, 42);
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

#### `this` argument

By default, the JS function has a `this` value of the object from which the
function is accessed (i.e. `&self`).

Alternatively, you can explicitly declare the type of `this` using the first
argument:

- `this: undefined`

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

- `this: [SomeType]`

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

  This allows you to explicitly pass a value as `this` when calling the
  function, which will be subject to the same
  [type conversion](#type-conversions) rules as normal arguments.

#### Option: #[js(func(`name = "..."`))]

Overrides the property key when accessing the function. Has the same usage as
[`#[js(prop(name))]`](#jspropname--).

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

#### Option: #[js(func(`Symbol(...)`))]

Overrides the property key when accessing the function. Has the same usage as
[`#[js(prop(Symbol))]`](#jspropsymbol).

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

### `#[js(new)]`

Generate a Rust function to invoke a JavaScript constructor.

```rust
# use ferrosaur::js;
# #[path = "../tests/util/mod.rs"]
# mod util;
#
# #[js(global_this)]
# struct Global;
#
#[js(interface)]
impl Global {
    #[js(new)]
    fn date(&self, timestamp: serde<f64>) -> Date {}
}
# #[js(value)]
# struct Date;
# #[js(interface)]
# impl Date {
#     #[js(func(name(toISOString)))]
#     fn to_iso_string(&self) -> serde<String> {}
# }
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut util::deno()?;
// let rt: &mut JsRuntime;
# let global = Global::new(rt);
// let global: Global;
let date = global.date(0.0, rt)?;
assert_eq!(date.to_iso_string(rt)?, "1970-01-01T00:00:00.000Z");
// struct Date; impl Date { ... }
# Ok(())
# }
```

```ts
let date = new Date(0);
assert(date.toISOString() === "1970-01-01T00:00:00.000Z");
```

The generated function has the signature

<!-- deno-fmt-ignore-start -->
> <span class="code-header">fn <span class="fn">\[name]</span>(&self, \[args...,] _rt: &mut [JsRuntime]) -> [anyhow::Result]\<...></span>
<!-- deno-fmt-ignore-end -->

This accepts the same argument and return type format as
[`#[js(func)]`](#jsfunc). Note that constructors cannot be async.

Implicitly, the class name is the name of the return type (with case preserved).
If the return type name cannot be used, such as if it is not a simple
identifier, or if you would like to override it, you can use the
[`class`](#jsnewclass) option.

#### Options <!-- omit from toc -->

- [`class(...)`](#jsnewclass)

#### Placement

Note that the return type indicates _the JavaScript type that will be
constructed_, whereas `&self` represents _the JavaScript object that has access
to the constructor_ (such as a module or `globalThis`).

In other words, the following usage is almost never correct:

```rust,compile_fail
#[js(interface)]
struct Rectangle {
    #[js(new)]
    fn new() -> Self {}
    // or
    #[js(new)]
    fn new(&self) -> Self {}
}
```

#### Option: #[js(new(`class(...)`))]

Use the specified key when accessing the constructor. Has the same usage as
[`#[js(prop(name))]`](#jspropname--).

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

### `#[js(get_index)]`, `#[js(set_index)]`

## `#[js(function)]`

## `#[js(iterator)]`

## Type conversions

### `Self` type

### Variadic arguments

# Guides

## Having modules import from each other

## Classes with static properties

## Emulating inheritance

<script type="module">
  import { codeToHtml } from "https://esm.sh/shiki@3.1.0";

  const parser = new DOMParser();

  document.querySelectorAll("pre.language-ts").forEach(async (elem) => {
    const text = elem.firstChild.textContent;

    const highlighted = await codeToHtml(text, {
      lang: "ts",
      themes: {
        light: "github-light",
        dark: "github-dark",
        ayu: "ayu-dark",
      },
      colorReplacements: {
        "github-light": {
          "#fff": "var(--code-block-background-color)",
        },
        "github-dark": {
          "#24292e": "var(--code-block-background-color)",
        },
        "ayu-dark": {
          "#0b0e14": "var(--code-block-background-color)",
        },
      },
      defaultColor: false,
    });

    const parsed = parser.parseFromString(highlighted, "text/html");
    elem.replaceWith(parsed.body.firstChild);
  });
</script>

<style>
  html[data-theme="light"] .shiki,
  html[data-theme="light"] .shiki span {
    background-color: var(--shiki-light-bg);
    color: var(--shiki-light);
  }

  html[data-theme="dark"] .shiki,
  html[data-theme="dark"] .shiki span {
    background-color: var(--shiki-dark-bg);
    color: var(--shiki-dark);
  }

  html[data-theme="ayu"] .shiki,
  html[data-theme="ayu"] .shiki span {
    background-color: var(--shiki-ayu-bg);
    color: var(--shiki-ayu);
  }
</style>
