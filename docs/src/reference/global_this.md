# `#[js(global_this)]`

Generate a [newtype] struct to hold a reference to [`globalThis`][globalThis].

```rust
# use ferrosaur::js;
#[js(global_this)]
struct Global;
```

Call the associated `new` function to initialize it given a `&mut JsRuntime`:

```rust
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
# use fixture::items::global::Global;
#
# let rt = &mut fixture::deno()?;
#
// let rt: &mut JsRuntime;
Global::new(rt);
#
# Ok::<_, anyhow::Error>(())
```

[newtype]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
[globalThis]: http://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/globalThis

## Trait Implementations

<!-- deno-fmt-ignore-start -->

<span class="code-header">impl [AsRef]<[v8::Global]<[v8::Object]>> for Global</span>

<span class="code-header">impl<\'a> [ToV8]<\'a> for Global</span>

<span class="code-header">impl<\'a> [ToV8]<\'a> for &\'_ Global</span>

<!-- deno-fmt-ignore-end -->

[v8::Global]: deno_core::v8::Global
[v8::Object]: deno_core::v8::Object
[ToV8]: deno_core::ToV8
