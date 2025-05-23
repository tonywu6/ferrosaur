# Specifying types

To be able to interact with JavaScript, code generated by this crate must convert
between Rust types and [V8][v8] data types. Function arguments and return types must
implement specific conversion traits.

<details class="toc" open>
  <summary>Sections</summary>

- [`ToV8` and `FromV8`](#tov8-and-fromv8)
- [`Serialize` and `DeserializeOwned`](#serialize-and-deserializeowned)

</details>

## `ToV8` and `FromV8`

[`ToV8`] and [`FromV8`] are [`deno_core`]'s builtin conversion traits.

**By default:**

- Function arguments must implement [`ToV8`];

  - In the case of [variadic functions](interface/func.md#spread-arguments), the
    argument must implement [`Iterator<Item = T>`], and `T` must implement [`ToV8`];

- Function return types, property accessor return types, and iterator item types must
  implement [`FromV8`].

In addition to existing implementors,

- Types derived with [`js(value)`](value.md), [`js(module)`](module.md), and
  [`js(global_this)`](global-this.md) implement [`ToV8`]. This means you can pass such
  values to JS functions as arguments.

- Types derived with [`js(value)`](value.md) implement [`FromV8`]. This means you can
  return such values from JS functions.

## `Serialize` and `DeserializeOwned`

Alternatively, you can opt in to data conversion using [`serde_v8`]. To do so, wrap the
type in `serde<...>`:

```rust
# use ferrosaur::js;
#
# #[js(value)]
# struct Foo;
#
use deno_core::serde::{Serialize, de::DeserializeOwned};

#[js(interface)]
impl Foo {
    #[js(func)]
    fn bar<T, U>(&self, baz: serde<T>) -> serde<U>
    where
        T: Serialize,
        U: DeserializeOwned,
    {}
}
```

In this case:

- Function arguments must implement [`Serialize`];

  - In the case of [variadic functions](interface/func.md#spread-arguments), the
    argument must implement [`Iterator<Item = T>`], and `T` must implement
    [`Serialize`];

- Function return types, property accessor return types, and iterator item types must
  implement [`DeserializeOwned`].
