# `#[js(interface)]`

Use `#[js(interface)]` to declare:

- [property accessors](interface/prop.md)
- [functions and methods](interface/func.md)
- [constructors](interface/new.md)
- [indexing getters and setters](interface/get-set.md)

You can use `js(interface)` on any type derived using this crate, such as a
[`js(value)`](value.md) or a [`js(module)`](module.md). You can even use it on traits,
see the [`ts` example](../examples/ts.md#declaring-interfaces-for-libts).

```rust
# use ferrosaur::js;
// First, declare a type:
#[js(value)]
struct CowSay;

// Then, declare its APIs:
#[js(interface)]
impl CowSay {
    #[js(prop)]
    fn moo(&self) -> String {}
}
```

## Example: The To-do List

Let's say you have the following JavaScript:

<figure>

```js
{{#include _snippets/todo-list.js}}
```

  <figcaption>../examples/js/mod.js</figcaption>
</figure>

Expressed in [TypeScript declarations][declarations], this is:

<figure>

```ts
{{#include _snippets/todo-list.d.ts}}
```

  <figcaption>../examples/js/mod.d.ts</figcaption>
</figure>

You can then express this in Rust as:

<figure>

{{#include _snippets/todo-list.md}}

</figure>

<!-- prettier-ignore-start -->

[declarations]: https://www.typescriptlang.org/docs/handbook/declaration-files/introduction.html

<!-- prettier-ignore-end -->
