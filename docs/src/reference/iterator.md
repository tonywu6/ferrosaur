# `#[js(iterator)]`

Use `#[js(iterator)]` to represent and interact with objects conforming to the [iterator
protocol][js-iterator].

<figure>

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
#
# #[js(value)]
# struct MapEntries;
#
#[js(iterator)]
impl MapEntries {
    type Item = serde<(String, String)>;
}
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;

// let rt: &mut JsRuntime;
let mut entries: MapEntries = rt
    .execute_script("eval", "new Map([['foo', 'bar']]).entries()")?
    .into();

let Ok(Some((k, v))) = entries.next(rt) else { panic!() };
assert_eq!(k, "foo");
assert_eq!(v, "bar");
#
# Ok(())
# }
```

```ts
// Expressed in TypeScript:
const entries = new Map([["foo", "bar"]]).entries();
const [k, v] = entries.next().value!;
assert(k === "foo");
assert(v === "bar");
```

</figure>

Use `js(callable)` on an `impl` block. The `impl` must contain a single item,
`type Item = T`, where `T` must implement either [`FromV8`] (the default) or
[`DeserializeOwned`] (if written as `serde<T>`).

> [!NOTE]
>
> See [Specifying types](typing.md) for more info on how you can specify types when
> using this crate.

## Derived APIs

> [!NOTE]
>
> Note that `js(iterator)` types do _not_ implement the [`Iterator`] trait: both of the
> below provided APIs require passing in a [`JsRuntime`] at call time.

### Methods

<div class="code-header">

#### pub fn next(&mut self, &mut [JsRuntime]) -> [anyhow::Result]\<Option\<T>>

Receive the next value from the iterator:

- `Ok(Some(T))` if the next value is successfully returned;
- `Ok(None)` if the iterator has been exhausted;
- `Err(...)` if there was an error while advancing the iterator.

More specifically, the returned value depends on the value returned by the underlying
`next()` function:

| JavaScript value                    | Rust value    |
| ----------------------------------- | ------------- |
| `{ done?: false, value: T }`        | `Ok(Some(T))` |
| `{ done: true, value: T }`          | `Ok(Some(T))` |
| `{ done: true, value?: undefined }` | `Ok(None)`    |
| Exception caught                    | `Err(...)`    |

#### pub fn into_iter<'a>(self, rt: &'a mut [JsRuntime]) <br> -> impl [Iterator]\<Item = [anyhow::Result]\<T>> + use\<'a>

Get a proper Rust [`Iterator`], which produces [`anyhow::Result<T>`].

This enables you to use all the capabilities of a Rust iterator, such as
[`collect()`][Iterator::collect()], as well as using it in a `for` loop.

> [!NOTE]
>
> Due to lifetime restrictions, the returned iterator mutably borrows the [`JsRuntime`]
> for the entire duration of the iteration. This will prevent you from using it on the
> produced items until the iterator is dropped.
>
> To be able to use the runtime during iteration, manually call
> [`next(&mut JsRuntime)`](#pub-fn-nextmut-self-mut-jsruntime---anyhowresultoptiont).

</div>

<!-- prettier-ignore-start -->

[js-iterator]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Iteration_protocols#the_iterator_protocol

<!-- prettier-ignore-end -->
