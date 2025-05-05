// ## Embed `ts-blank-space`

use ferrosaur::js;

#[js(module("../dist/main.js", fast))]
struct Main;

#[js(interface)]
impl Main {
    #[js(func(name = "default"))]
    fn blank_space<S: serde::Serialize>(&self, src: serde<S>) -> serde<String> {}
    // import { default as blank_space } from "../dist/main.js";
}

// The file `../dist/main.js` is emitted by [esbuild] during `cargo build`.
//
// See [`build.ts`](/examples/ts-blank-space/build.ts) which slightly processes the
// `ts-blank-space` library so that it can be used in this example.

// ## Setup the runtime

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut deno()?;

    // ## Initialize `typescript`

    use example_ts::{inject_env_vars, TypeScriptLib, TypeScriptVfs};

    TypeScriptLib::side_module_init(rt).await?;

    TypeScriptVfs::side_module_init(rt).await?;

    // `TypeScriptLib` and `TypeScriptVfs` are provided by the [`ts` example](/docs/src/examples/ts.md#srclibrs).

    inject_env_vars(rt)?;

    // `inject_env_vars` sets up some data that `typescript` requires in order to run.
    // See [`build.rs` in the `ts` example](/docs/src/examples/ts.md#buildrs) for more info.

    // ## Initialize `ts-blank-space`

    let ts = Main::main_module_init(rt).await?;

    // ## Run `ts-blank-space` on [`examples/ts/src/lib.ts`](/docs/src/examples/ts.md#srclibts)

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ts/src/lib.ts");

    let file = std::fs::read_to_string(&path)?;

    let js = ts.blank_space(&file, rt)?;

    // ## Evaluate the type-stripped result

    #[js(value(of_type(v8::Object)))]
    struct Example;

    let module: Example = {
        let url = Url::from_directory_path(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .join("ad-hoc.js")?;
        let id = rt.load_side_es_module_from_code(&url, js.clone()).await?;
        rt.mod_evaluate(id).await?;
        rt.get_module_namespace(id)?.into()
    };

    use example_ts::Compiler;

    impl Compiler for Example {}

    // `example_ts::Compiler` [describes the JavaScript APIs](/docs/src/examples/ts.md#declaring-interfaces-for-libts)
    // exported by [`lib.ts`](/docs/src/examples/ts.md#srclibts).

    // Here we are saying `Example`, our ad-hoc ES module produced by `ts-blank-space`, comforms
    // to the interface as described by the `Compiler` trait, which is correct.

    // ## Pretty-print the type-stripped result

    use bat::PrettyPrinter;

    PrettyPrinter::new()
        .input_from_bytes(js.as_bytes())
        .language("javascript")
        .theme("GitHub")
        .print()?;

    println!();

    // `PrettyPrinter` courtesy of [`bat`](https://crates.io/crates/bat).

    // ## Use `lib.ts` to type check itself

    let root = HashMap::new().tap_mut(|map| drop(map.insert("src/lib.ts".into(), file)));

    let errors = module
        .create_program(root, rt)?
        .print_diagnostics(true, rt)?;

    println!("{errors}");

    {
        let mut settings = insta::Settings::clone_current();
        settings.set_description("script compiled with ts-blank-space");
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_path("../tests/snapshots");
        settings.bind(|| insta::assert_snapshot!(js));
    }

    Ok(())
}

// <details>
//   <summary>Additional setup code</summary>

use std::{collections::HashMap, path::Path};

use anyhow::Result;
use tap::Tap;

use example_runtime::{
    deno,
    deno_core::{self, serde, url::Url},
};

// </details>

// [esbuild]: https://esbuild.github.io/
