// `lib.rs` does the following things:
//
// - Embed the `typescript` library and export it as the `TypeScript` struct.
// - Provide some reusable interface definitions and utility functions.
//
// The [`ts-blank-space`](/docs/src/examples/ts-blank-space.md) example reuses
// this module because it also requires `typescript`.

// ### Embedding `typescript`

use ferrosaur::js;

#[js(module("../dist/typescript.js", url("npm:typescript"), fast(unsafe_debug)))]
pub struct TypeScript;

// - The actual embedded file is `"../dist/typescript.js"`. This file is emitted by
//   [esbuild] during the build step. The actual source file is [`lib.ts`](#srclibts).
//   See also [`build.rs`](#buildrs) and [`build.js`](#buildjs).
//
// - `url("npm:typescript")` [sets the module specifier][module-url] to `"npm:typescript"`.
//
//   [`lib.ts`](#srclibts) and other modules in the runtime will then be able to do <br>
//   `import ts from "npm:typescript"`.
//
// - `fast(unsafe_debug)` embeds the JS file as a fast V8 string while
//   [skipping compile-time assertion][module-fast-unsafe] that it is in ASCII. This is
//   because the `typescript` lib is massive and doing so will take a long time.
//
//   `esbuild` already [ensures that its build output is ASCII-only][esbuild-charset],
//   so it is safe in this case.

// ### Declaring interfaces for [`lib.ts`](#srclibts)

#[js(interface)]
pub trait Compiler {
    #[js(func)]
    fn create_program(&self, ..files: serde<Vec<String>>) -> Program {}
}

#[js(value)]
pub struct Program;

#[js(interface)]
impl Program {
    #[js(func)]
    pub fn print_diagnostics(&self, colored: bool) -> serde<String> {}
}

// [`#[js(interface)]`][js-interface] is being used on a trait `Compiler` here. This
// turns `Compiler` into sort of a [marker trait][marker-trait],
// and enables a form of [duck typing][duck-typing].

// It is essentially saying "any Rust type that implements `Compiler` will provide the
// `create_program` function." For example, the `Example` struct, which embeds [`lib.ts`](#srclibts):

#[js(module("../dist/lib.js", fast))]
pub struct Example;

impl Compiler for Example {}

// Of course, _ferrosaur_ cannot actually verify such an implementation, so it is up to
// the programmer to guarantee that implementors of such traits actually provide the
// specified interfaces.

// ### Helpers

// <details>
//   <summary><code>inject_env_vars</code></summary>

// This function defines a few properties on `globalThis` to be used in the example.

pub fn inject_env_vars(rt: &mut JsRuntime) -> Result<()> {
    #[js(global_this)]
    struct Global;

    #[js(interface)]
    impl Global {
        #[js(set_index)]
        fn define_object(&self, name: serde<&str>, value: v8::Global<v8::Object>) {}

        #[js(set_index)]
        fn define_string(&self, name: serde<&str>, value: serde<&str>) {}
    }

    let global = Global::new(rt);

    let dts = {
        let scope = &mut rt.handle_scope();
        dts::dts(scope)?
    };

    global.define_object("TYPESCRIPT_LIB", dts, rt)?;
    global.define_string("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"), rt)?;

    Ok(())
}

// </details>

// <details>
//   <summary><code>mod dts</code></summary>

// See [build.rs](#buildrs).

mod dts {
    include!(concat!(env!("OUT_DIR"), "/lib.dts.rs"));
}

// </details>

// <details>
//   <summary>Additional setup code</summary>

use anyhow::Result;

use example_runtime::deno_core::{self, JsRuntime};

// </details>

// [esbuild]:               https://esbuild.github.io/
// [duck-typing]:           https://en.wikipedia.org/wiki/Duck_typing
// [esbuild-charset]:       https://esbuild.github.io/api/#charset
// [js-interface]:          /docs/src/reference/interface.md
// [marker-trait]:          https://doc.rust-lang.org/nomicon/send-and-sync.html?highlight=marker
// [module-url]:            /docs/src/reference/module.md#url
// [module-fast-unsafe]:    /docs/src/reference/module.md#fastunsafe_debug
