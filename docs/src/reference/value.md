# `#[js(value)]`

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

## Options <!-- omit from toc -->

- [`of_type(T)`](#jsvalueof_typet)

## Trait Implementations

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

## Option: #\[js(value(`of_type(T)`))]

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
