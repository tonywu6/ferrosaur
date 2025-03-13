# `#[js(module)]` <!-- omit from toc -->

Embed an [ECMAScript module][esm] in the program.

```rust
# use ferrosaur::js;
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
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
# use fixture::items::modules::{Main, Iter as Side};
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
let main = Main::main_module_init(rt).await?;
let side = Side::side_module_init(rt).await?;
#
# Ok(())
# }
```

[esm]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules
[load_main_es_module_from_code]: deno_core::JsRuntime::load_main_es_module_from_code
[load_side_es_module_from_code]: deno_core::JsRuntime::load_side_es_module_from_code
[`include_str!`]: https://doc.rust-lang.org/stable/core/macro.include_str.html

<details class="toc" open>
  <summary>Table of contents</summary>

- [Option `fast`](#option-fast)
- [Option `fast(unsafe_debug)`](#option-fastunsafe_debug)
- [Option `url(...)`](#option-url)
  - [Option `url(preserve)`](#option-urlpreserve)
  - [Option `url(cwd)`](#option-urlcwd)
  - [Option `url("...")`](#option-url-1)
- [Trait implementations](#trait-implementations)

</details>

## Option `fast`

Syntax: `#[js(module("...", fast))]`

Without the `fast` option, JavaScript source code is embedded using
[`include_str!`].

With the `fast` option, JavaScript source code is embedded using
[`deno_core::ascii_str_include!`][deno_core::ascii_str_include] instead.

**The JS source file must be in 7-bit ASCII.** It is a compile error if this is
not true.

See [`FastStaticString`][FastStaticString] and
[`ascii_str_include!`][deno_core::ascii_str_include] for more info.

[FastStaticString]: deno_core::FastStaticString

## Option `fast(unsafe_debug)`

Syntax: `#[js(module("...", fast(unsafe_debug)))]`

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

## Option `url(...)`

Syntax: `#[js(module("...", url(...)))]`

Controls the [`import.meta.url`][import-meta-url] within the module.

Choices are:

### Option `url(preserve)`

[`import.meta.url`][import-meta-url] will be `file:///` + relative path to
[`CARGO_MANIFEST_DIR`][cargo-env].

For example, if the JS file is at `src/js/index.js`, then the URL is
`"file:///src/js/index.js"`

### Option `url(cwd)`

[`import.meta.url`][import-meta-url] will be `file://` +
[`std::env::current_dir()`] _at runtime_ + a name generated from the file's
relative path.

For example, if the JS file is at `src/js/index.js`, and the program's runtime
working directory is `/path/to/cwd`, then the URL is
`"file:///path/to/cwd/-src-js-index.js"`

### Option `url("...")`

Use a custom [`import.meta.url`][import-meta-url]. The string must be parseable
by [`url::Url`][deno_core::url::Url]. It is a runtime error if the URL is not
valid.

For example, `url("npm:lodash")` causes `import.meta.url` to be `"npm:lodash"`.

See the [Guides][TODO:] section for the uses for these options.

[import-meta-url]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta#url
[cargo-env]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates

## Trait implementations

<!-- deno-fmt-ignore-start -->

<span class="code-header">impl [AsRef]<[v8::Global]<[v8::Object]>> for Module</span>

<span class="code-header">impl<\'a> [ToV8]<\'a> for Module</span>

<span class="code-header">impl<\'a> [ToV8]<\'a> for &\'_ Module</span>

<!-- deno-fmt-ignore-end -->

[ToV8]: deno_core::ToV8
[v8::Global]: deno_core::v8::Global
[v8::Object]: deno_core::v8::Object
