# `#[js(callable)]`

Use `#[js(callable)]` to store JavaScript functions as values while declaring their
signatures.

This could be used to describe functions that return other functions:

<figure>

```rust
# use ferrosaur::js;
#
# #[js(value)]
# struct Logging;
#
#[js(interface)]
impl Logging {
    #[js(func)]
    fn with_prefix(&self, prefix: serde<&str>) -> Logger {}
}
#
# #[js(value)]
# struct Logger;
#
#[js(callable)]
impl Logger {
    fn call(&self, message: serde<&str>) {}
}
```

```ts
// Expressed in TypeScript:
interface Logging {
  with_prefix: (prefix: string) => Logging;
  //           (prefix: string) => (message: string) => void
}
interface Logger {
  (message: string): void;
}
```

</figure>

Use `js(callable)` on an `impl` block, which must contain a single item, a function
named `call`, whose signature follows the same usage as [`js(func)`](interface/func.md).

> [!TIP]
>
> JavaScript does not have a proper "callable interface." This is named `callable` to
> distinguish it from [`js(func)`](interface/func.md), which is for describing named
> functions accessible from an object.
