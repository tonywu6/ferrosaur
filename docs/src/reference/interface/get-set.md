# `#[js(get_index)]`, `#[js(set_index)]`

Use `#[js(get_index)]` and `#[js(set_index)]` for dynamic property access and update
(i.e. the `obj[prop]` bracket notation).

<figure>

```rust
# use ferrosaur::js;
#
# #[path = "../../../crates/ferrosaur/tests/fixture/mod.rs"]
# mod fixture;
#
# #[js(value)]
# struct Record;
#
#[js(interface)]
impl Record {
    #[js(get_index)]
    fn get(&self, k: serde<&str>) -> String {}
    #[js(set_index)]
    fn set(&self, k: serde<&str>, v: serde<&str>) {}
}
#
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let rt = &mut fixture::deno()?;
// let rt: &mut JsRuntime;
#
# let record: Record = fixture::eval_value("({})", rt)?;
// let record: Record;
#
let key = "foo";
record.set(key, "bar", rt)?;
assert_eq!(record.get(key, rt)?, "bar");
#
# Ok(())
# }
```

```ts
// Expressed in TypeScript:
declare const record: Record<string, string>;
const key = "foo";
record[key] = "bar";
assert(record[key] === "bar");
```

</figure>

Functions decorated with `js(get_index)` must have 2 arguments: `&self` and the key to
get, as well as a return type: the type of the value. Functions decorated with
`js(get_index)` must have 3 arguments: `&self`, the key, and the value to set.

Argument types must implement either [`ToV8`] (the default) or [`Serialize`] (if written
as `serde<T>`). The return type must implement either [`FromV8`] or
[`DeserializeOwned`].

> [!NOTE]
>
> See [Specifying types](../typing.md) for more info on how you can specify types when
> using this crate.
