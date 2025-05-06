# Common pitfalls

- [the trait bound `ToV8<'_>`/`FromV8<'_>` is not satisfied](#the-trait-bound-tov8_fromv8_-is-not-satisfied)
- [use of undeclared crate or module `deno_core`](#use-of-undeclared-crate-or-module-deno_core)

## the trait bound `ToV8<'_>`/`FromV8<'_>` is not satisfied

Function argument and return types for [`js(func)`](interface/func.md),
[`js(prop)`](interface/prop.md), etc., must implement specific traits for data to be
able to pass to/from JavaScript.

By default, arguments must implement [`ToV8`], and return types must implement
[`FromV8`].

If you would like to serialize data using [`serde`] instead, you can opt into this
behavior by rewriting a type `T` as `serde<T>`:

```diff
  #[js(interface)]
  impl Foo {
      #[js(func)]
-     fn bar(&self, baz: Baz) {}
+     fn bar(&self, baz: serde<Baz>) {}
  }
```

See [Specifying types](typing.md) for additional information on data conversion.

## use of undeclared crate or module `deno_core`

The `#[js]` macro generates code that references the [`deno_core`] crate, but it does
not generate a `use deno_core` statement. Instead, it assume that `deno_core` is in
scope where the macro is used.

If your crate directly depends on `deno_core`, then the macro will work without extra
steps. If your crate does not directly depend on `deno_core`, for example, if you are
using `deno_runtime` instead, then you can manually introduce `deno_core` into scope:

```diff
+ use deno_runtime::deno_core;

  #[js(global_this)]
  struct Global;
```
