# `#[js(interface)]`

Declare JavaScript "interfaces":

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
# use fixture::items::global::Global;
#
#[js(interface)]
impl Global {
    // ...
}
```

You can declare:

- [`#[js(prop)]`](interface/prop.md) — property accessors
- [`#[js(func)]`](interface/func.md) — functions
- [`#[js(new)]`](interface/new.md) — constructors
- [`#[js(get_index)]`, `#[js(set_index)]`](interface/get-set.md) — indexing
  getters and setters

You can use `js(interface)` to generate implementations for any Rust types
generated using this crate, see [`Self` type][TODO:] for more info. You can even
use `js(interface)` on traits, see [Emulating inheritance][TODO:].
