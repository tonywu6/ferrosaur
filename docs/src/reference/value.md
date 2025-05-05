# `#[js(value)]`

Use `#[js(value)]` to represent arbitrary JavaScript values as types in Rust's type
system.

```rust
# use ferrosaur::js;
#[js(value)]
struct Lorem;
```

The derived types are not intended to be instantiated directly. Instead, you can return
them from APIs that you declare on [`js(global_this)`](global-this.md), a
[`js(module)`](module.md), or another `js(value)`. To declare APIs, use
[`js(interface)`](interface.md).

<details>
  <summary>Illustrative example: The To-do List</summary>

{{#include _snippets/todo-list.md}}

</details>

> [!TIP]
>
> Types derived with `js(value)`, `js(module)`, and `js(global_this)` are essentially
> _[newtypes]_ around V8 types.

<details class="toc" open>
  <summary>Sections</summary>

- [Option `of_type(T)`](#option-of_typet)
- [Derived APIs](#derived-apis)

</details>

## Option `of_type(T)`

By default, `js(value)` generates a struct that is:

```rust
use deno_core::v8;
struct Lorem(v8::Global<v8::Value>);
//                          ^ inner type
```

By using the `of_type` option, you can use some other V8 data types for the inner type.
For example:

```rust
# use ferrosaur::js;
#[js(value(of_type(v8::Promise)))]
struct Response;
// struct Response(v8::Global<v8::Promise>);
```

It should make sense for the data type `T` to be placed in a [`v8::Global`]. In
particular, this means [`v8::Local<v8::Value>`] implements `TryInto<v8::Local<T>>`.

This could be useful if you want to have simple runtime type checking for your types.
For example, given the `Response` type above, if a JS function is supposed to return a
`Response`, i.e. a `Promise`, but it returns `undefined`, then the corresponding Rust
function returns `Err(...)` instead of `Ok(Response)`.

Note that this is "type checking" only in so far as [`v8`] can try-convert between
different V8 types; this is not TypeScript-style structural typing.

> [!NOTE]
>
> See [Specifying types](typing.md) for more info on how you can specify types when
> using this crate.

## Derived APIs

In the signatures below,

- `Type` is the type that you apply `js(value)` to;
- `<T>` is the one of the `v8::*` data types. By default, this is [`v8::Value`], but you
  can control it using the [`of_type`](#option-of_typet) option.

### Trait implementations

<div class="code-header">

#### impl [AsRef]<[v8::Global]\<T>> for Type

#### impl [From]<[v8::Global]\<T>> for Type

#### impl [From]\<Type> for [v8::Global]\<T>

#### impl<'a> [FromV8]<'a> for Type

#### impl<'a> [ToV8]<'a> for Type

#### impl<'a> [ToV8]<'a> for &'\_ Type

</div>

<!-- prettier-ignore-start -->

[newtypes]: https://doc.rust-lang.org/book/ch20-03-advanced-types.html#using-the-newtype-pattern-for-type-safety-and-abstraction

<!-- prettier-ignore-end -->
