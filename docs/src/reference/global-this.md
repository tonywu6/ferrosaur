# `#[js(global_this)]`

Use `#[js(global_this)]` for access to the JavaScript [`globalThis`] object:

```rust
# use ferrosaur::js;
// use it on a unit struct:
#[js(global_this)]
struct Global;
// (struct name does not need to be `Global`)
```

Call the `new` method to initialize it:

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
# use fixture::items::global::Global;
#
# let rt = &mut fixture::deno()?;
#
// let rt: &mut JsRuntime;
let global = Global::new(rt);
#
# Ok::<_, anyhow::Error>(())
```

After this, you can use [`#[js(interface)]`](interface.md) to further derive access to
[properties](interface/prop.md), [functions](interface/func.md), and more, on
`globalThis`:

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
# use fixture::items::global::Global;
#
#[js(interface)]
impl Global {
    #[js(func)]
    fn atob(&self, to_decode: String) -> String {}
}
#
# Ok::<_, anyhow::Error>(())
```

## Derived APIs

### Methods

<div class="code-header">

#### pub fn new(rt: &mut [JsRuntime]) -> Self

Create a handle to the [`globalThis`] object from the given [`JsRuntime`].

</div>

### Trait implementations

<div class="code-header">

#### impl [AsRef]<[v8::Global]<[v8::Object]>> for Global

#### impl<'a> [ToV8]<'a> for Global

#### impl<'a> [ToV8]<'a> for &'\_ Global

</div>

<!-- prettier-ignore-start -->

[`globalThis`]: http://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/globalThis
[newtype]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html

<!-- prettier-ignore-end -->
