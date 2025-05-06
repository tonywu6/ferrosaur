// `lib.rs` does the following things:
//
// - Embed JS dependencies and export them as Rust structs.
// - Provide some reusable interface definitions and utility functions.
//
// The [`ts-blank-space`](/docs/src/examples/ts-blank-space.md) example reuses
// this module because they also require `typescript`.

// ### Embedding `typescript`

use ferrosaur::js;

#[js(module("../dist/typescript.js", url("npm:typescript"), fast(unsafe_debug)))]
pub struct TypeScriptLib;

// - The embedded file is `"../dist/typescript.js"`. This file is emitted by [esbuild]
//   during `cargo build`. The actual source file is under [`src/deps`](#srcdeps).
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
//   esbuild already [ensures that its build output is ASCII-only][esbuild-charset],
//   so it is safe in this case.

// ### Embedding `@typescript/vfs`

#[js(module(
    "../dist/typescript-vfs.js",
    url("npm:@typescript/vfs"),
    fast(unsafe_debug)
))]
pub struct TypeScriptVfs;

// This example additionally uses [`@typescript/vfs`] to setup the files necessary
// for `typescript` to type check. These files are also embedded into the program,
// albeit via a dedicated [build step](#buildrs).

// [`@typescript/vfs`] is the same system that enables the [TypeScript playground](https://www.typescriptlang.org/play/)
// to run in the browser.

// ### Declaring interfaces for [`lib.ts`](#srclibts)

#[js(interface)]
pub trait Compiler {
    #[js(func)]
    fn create_program(&self, root: serde<HashMap<String, String>>) -> Program {}
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
//   <summary><code>inject_lib_dts</code></summary>

// This function defines a few properties on `globalThis` to be used in the example.

// Notably, it injects `TYPESCRIPT_LIB`. On the Rust side, this is the [embedded](#buildrs)
// declaration files. On the JavaScript side, this is used to create the
// [virtual file system](#embedding-typescriptvfs).

pub fn inject_lib_dts(rt: &mut JsRuntime) -> Result<()> {
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

// `lib.dts.rs` is the generated file that [embeds](#buildrs) declaration files.
// See [build.rs](#buildrs).

mod dts {
    include!(concat!(env!("OUT_DIR"), "/lib.dts.rs"));
}

// </details>

// <details>
//   <summary>Additional setup code</summary>

use std::collections::HashMap;

use anyhow::Result;

use example_runtime::deno_core::{self, JsRuntime};

// </details>

// [`@typescript/vfs`]:     https://www.npmjs.com/package/@typescript/vfs
// [esbuild]:               https://esbuild.github.io/
// [duck-typing]:           https://en.wikipedia.org/wiki/Duck_typing
// [esbuild-charset]:       https://esbuild.github.io/api/#charset
// [js-interface]:          /docs/src/reference/interface.md
// [marker-trait]:          https://doc.rust-lang.org/nomicon/send-and-sync.html?highlight=marker
// [module-url]:            /docs/src/reference/module.md#url
// [module-fast-unsafe]:    /docs/src/reference/module.md#fastunsafe_debug
