# Example: TypeScript

This example showcases a setup of multiple ES modules that can import from each other,
as well as integration with JavaScript tooling during `cargo build`.

<div class="hidden">

> [!TIP]
>
> For best results, view this page
> [in the book](https://tonywu6.github.io/ferrosaur/examples/ts).

</div>

It [embeds][js-module] the [`typescript`] compiler in order to type check the example
source code itself.

To run this example, run:

    cargo run --package example-ts

> [!IMPORTANT]
>
> This example requires the [`deno`] CLI to build.

<figure>

![screenshot of the example](/docs/src/examples/media/ts.webp)

<figcaption>

Output of this example. Errors are expected since `typescript` as used in this example
actually does not have access to these types.

</figcaption>

</figure>

> [!TIP]
>
> See also [microsoft/typescript-go](https://github.com/microsoft/typescript-go), which
> makes this whole thing kinda silly.

<details class="toc" open>
  <summary>The setup</summary>

- [`src/lib.rs`](#srclibrs)
- [`src/lib.ts`](#srclibts)
- [`src/main.rs`](#srcmainrs)
- [`build.ts`](#buildts)
- [`build.rs`](#buildrs)
- [`src/deps/*`](#srcdeps)

</details>

## `src/lib.rs`

```rs,example
{{#include src/lib.rs}}
```

## `src/lib.ts`

`lib.ts` integrates `typescript` and implements functions used in this compiler. It is
embedded by [`lib.rs`](#srclibrs).

<details>
  <summary>File <code>src/lib.ts</code></summary>

```ts
{{#include src/lib.ts}}
```

</details>

## `src/main.rs`

```rs,example
{{#include src/main.rs}}
```

## `build.ts`

`build.ts` compiles TypeScript files to JavaScript using [esbuild] at the time of
`cargo build`.

Compiling is necessary because:

- [`deno_core`] itself does not run TypeScript files.

- The `typescript` lib is distributed in [CommonJS]. esbuild transforms it into ESM so
  that it can be [imported](#srclibrs).

<details>
  <summary>File <code>build.ts</code></summary>

```ts
{{#include build.ts}}
```

</details>

## `build.rs`

[`build.rs`][build-scripts] does a few things:

- Run [`build.ts`](#buildts) via [`deno`] to compile the `.ts` files used in this
  example.

- Generate Rust code that will embed TypeScript's [`lib` declarations][typescript-lib]
  into the program.

  These files contain definitions for ECMAScript language APIs, such as essential types
  like `Promise`, without which `typescript` will not know how to type check.

  The generated Rust functions are emitted under [`OUT_DIR`] and then included in
  [`lib.rs`](#srclibrs) using [`include!`].

<details>
  <summary>File <code>build.rs</code></summary>

```rs
{{#include build.rs}}
```

</details>

## `src/deps/*`

These files re-export the respective libs for [bundling](#buildts).

<details>
  <summary>File <code>src/deps/typescript.ts</code></summary>

```ts
{{#include src/deps/typescript.ts}}
```

</details>

<details>
  <summary>File <code>src/deps/typescript-vfs.ts</code></summary>

```ts
{{#include src/deps/typescript-vfs.ts}}
```

</details>

<!-- prettier-ignore-start -->

[CommonJS]:       https://nodejs.org/api/modules.html#modules-commonjs-modules
[`OUT_DIR`]:      https://doc.rust-lang.org/cargo/reference/environment-variables.html?highlight=OUT_DIR#environment-variables-cargo-sets-for-crates
[`deno`]:         https://docs.deno.com/runtime/
[`include!`]:     https://doc.rust-lang.org/std/macro.include.html
[`typescript`]:   https://www.npmjs.com/package/typescript
[build-scripts]:  https://doc.rust-lang.org/cargo/reference/build-scripts.html
[esbuild]:        https://esbuild.github.io/
[js-module]:      /docs/src/reference/module.md
[typescript-lib]: https://github.com/microsoft/TypeScript/blob/main/src/lib/README.md

<!-- prettier-ignore-end -->
