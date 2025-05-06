# Example: [`ts-blank-space`]

<div class="hidden">

> [!TIP]
>
> For best results, view this page
> [in the book](https://tonywu6.github.io/ferrosaur/examples/ts-blank-space).

</div>

[`ts-blank-space`] is a cool _type-stripping_ TypeScript compiler. This example builds
upon the [`ts` example](/docs/src/examples/ts.md) to run [`ts-blank-space`].

> [!TIP]
>
> "Type-stripping" means [erasing] TypeScript specific syntax and features from the
> source code so that it can be directly executed as JavaScript.

To run this example, run:

    cargo run --package example-ts-blank-space

> [!IMPORTANT]
>
> This example requires the [`deno`] CLI to build.

<figure>

![screenshot of the example](/docs/src/examples/media/ts-blank-space.webp)

<figcaption>

Output of this example. Notice the extra whitespace in declarations.

</figcaption>

</figure>

---

```rs,example
{{#include src/main.rs}}
```

<!-- prettier-ignore-start -->

[`deno`]:           https://docs.deno.com/runtime/
[`ts-blank-space`]: https://bloomberg.github.io/ts-blank-space/
[erasing]:          https://www.totaltypescript.com/erasable-syntax-only#what-does-erasable-mean

<!-- prettier-ignore-end -->
