# `#[js(module)]` <!-- omit from toc -->

Use `#[js(module)]` to embed an [ECMAScript module][esm] in the program.

```rust
# use ferrosaur::js;
#[js(module("../examples/js/mod.js"))]
pub struct Module;
```

The path is relative to the current file (it has the same usage as the [`include_str!`]
macro).

Call the
[`main_module_init`](#pub-async-fn-main_module_initrt-mut-jsruntime---anyhowresultself)
or
[`side_module_init`](#pub-async-fn-side_module_initrt-mut-jsruntime---anyhowresultself)
method to initialize it as a main module or side module in the given
[`JsRuntime`][JsRuntime].

```rust
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
# use fixture::items::modules::Main as MainModule;
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
let main = MainModule::main_module_init(rt).await?;
#
# Ok(())
# }
```

> [!NOTE]
>
> For the difference between a main module and a side module, see documentation for the
> corresponding [`JsRuntime`][JsRuntime] methods:
>
> - [`load_main_es_module_from_code`][load_main_es_module_from_code]
> - [`load_side_es_module_from_code`][load_main_es_module_from_code]

After this, you can use [`#[js(interface)]`](interface.md) to further derive access to
items exported from your module. For example, if you have:

```js
export const answer = "42";
```

Then you can write:

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
# use fixture::items::modules::{Main as MainModule};
#
#[js(interface)]
impl MainModule {
    #[js(prop)]
    fn answer(&self) -> String {}
}
#
# Ok::<_, anyhow::Error>(())
```

<details class="toc" open>
  <summary>Sections</summary>

- [Option `fast`](#option-fast)
  - [`fast(unsafe_debug)`](#fastunsafe_debug)
- [Option `url(...)`](#option-url)
  - [`url(preserve)`](#urlpreserve)
  - [`url(cwd)`](#urlcwd)
  - [`url("...")`](#url)
- [Derived APIs](#derived-apis)
  - [Methods](#methods)
  - [Associated items](#associated-items)
  - [Trait implementations](#trait-implementations)

</details>

## Option `fast`

```rust
# use ferrosaur::js;
#[js(module("../examples/js/mod.js", fast))]
pub struct Module;
```

Without the `fast` option, JavaScript source code is embedded using [`include_str!`].

With the `fast` option, JavaScript source code is embedded using
[`deno_core::ascii_str_include!`] instead.

**The JS source file must be in 7-bit ASCII.** It is a compile-time error if this does
not hold.

> [!NOTE]
>
> For what it means for the string to be "fast," from [`deno_core::FastStaticString`]:
>
> > A static string that is compile-time checked to be ASCII and is stored in the most
> > efficient possible way to create V8 strings.

### `fast(unsafe_debug)`

```rust
# use ferrosaur::js;
#[js(module("../examples/js/mod.js", fast(unsafe_debug)))]
pub struct Module;
```

Like [`fast`](#option-fast), except for debug builds, at compile time, _unsafely_ embeds
JS code as [`FastStaticString`][deno_core::FastStaticString]s [without
checking][unchecked] it is in 7-bit ASCII.

For release builds, this behaves the same as `fast`. Under the hood, this uses the
`#[cfg(debug_assertions)]` condition.

Behavior is undefined if the file is not actually in ASCII.

This could be useful if the source file you are trying to embed is very large, in which
case the compile-time checking could take a very long time.

## Option `url(...)`

Control the value of [`import.meta.url`][import-meta-url] within the module:

- [`url(preserve)`](#urlpreserve) <!-- no toc -->
- [`url(cwd)`](#urlcwd)
- [`url("...")`](#url)

### `url(preserve)`

```rust
# use ferrosaur::js;
#[js(module("../examples/js/mod.js", url(preserve)))]
pub struct Module;
```

`import.meta.url` will be `file:///` followed by the relative path from
[`CARGO_MANIFEST_DIR`][cargo-env] to the embedded JS file. This is the default behavior
if the `url(...)` option is not specified.

| Example           |                                        |
| :---------------- | -------------------------------------- |
| JavaScript file   | `<CARGO_MANIFEST_DIR>/src/js/index.js` |
| `import.meta.url` | `"file:///src/js/index.js"`            |

### `url(cwd)`

```rust
# use ferrosaur::js;
#[js(module("../examples/js/mod.js", url(cwd)))]
pub struct Module;
```

`import.meta.url` will be `file://` + [`std::env::current_dir()`] _at runtime_ + a name
generated from the file's relative path.

| Example           |                                          |
| :---------------- | ---------------------------------------- |
| JavaScript file   | `<CARGO_MANIFEST_DIR>/src/js/index.js`   |
| `current_dir()`   | `/path/to/cwd`                           |
| `import.meta.url` | `"file:///path/to/cwd/-src-js-index.js"` |

### `url("...")`

```rust
# use ferrosaur::js;
#[js(module("../examples/js/mod.js", url("...")))]
pub struct Module;
```

Use a custom `import.meta.url`.

The string must be parsable by [`url::Url`][Url]. It is a _runtime_ error if the URL is
not parsable. Notably, this means you cannot use a bare identifier like `"package"` as
you would with Node.

For example, `url("npm:lodash")` sets `import.meta.url` to `"npm:lodash"`.

## Derived APIs

### Methods

<div class="code-header">

#### pub async fn main_module_init(rt: &mut [JsRuntime]) -> [anyhow::Result]\<Self> <!-- omit from toc -->

Initialize the embedded ES module as a [main module][load_main_es_module_from_code] in
the given [`JsRuntime`][JsRuntime].

#### pub async fn side_module_init(rt: &mut [JsRuntime]) -> [anyhow::Result]\<Self> <!-- omit from toc -->

Initialize the embedded ES module as a [side module][load_side_es_module_from_code] in
the given [`JsRuntime`][JsRuntime].

#### pub fn module_url() -> [anyhow::Result]<[ModuleSpecifier][Url]> <!-- omit from toc -->

Get the `import.meta.url` within the module (controllable through the
[`url(...)`](#option-url) option).

</div>

### Associated items

<div class="code-header">

#### pub const MODULE_SRC: [&str][str] or [FastStaticString] <!-- omit from toc -->

The embedded JS source code as a constant.

</div>

### Trait implementations

<div class="code-header">

#### impl [AsRef]<[v8::Global]<[v8::Object]>> for Module <!-- omit from toc -->

#### impl<'a> [ToV8]<'a> for Module <!-- omit from toc -->

#### impl<'a> [ToV8]<'a> for &'\_ Module <!-- omit from toc -->

</div>

<!-- prettier-ignore-start -->

[FastStaticString]: deno_core::FastStaticString
[JsRuntime]: deno_core::JsRuntime
[ToV8]: deno_core::ToV8
[Url]: deno_core::url::Url
[`deno_core::ascii_str_include!`]: https://docs.rs/deno_core/0.338.0/deno_core/macro.ascii_str_include.html
[`include_str!`]: https://doc.rust-lang.org/stable/core/macro.include_str.html
[cargo-env]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
[esm]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules
[import-meta-url]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta#url
[load_main_es_module_from_code]: deno_core::JsRuntime::load_main_es_module_from_code
[load_side_es_module_from_code]: deno_core::JsRuntime::load_side_es_module_from_code
[unchecked]: deno_core::v8::String::create_external_onebyte_const_unchecked
[v8::Global]: deno_core::v8::Global
[v8::Object]: deno_core::v8::Object

<!-- prettier-ignore-end -->
