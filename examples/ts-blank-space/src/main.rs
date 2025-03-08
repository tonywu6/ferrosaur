use std::path::Path;

use anyhow::Result;
use bat::PrettyPrinter;
use ferrosaur::js;

use example_runtime::{
    deno,
    deno_core::{self, serde, url::Url},
};
use example_ts::{inject_env_vars, Compiler, TypeScript};

#[js(module("../dist/main.js", fast))]
struct Main;

#[js(interface)]
impl Main {
    #[js(func(name = "default"))]
    fn blank_space<S: serde::Serialize>(&self, src: serde<S>) -> serde<String> {}
}

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut deno(Main::module_url()?)?.js_runtime;

    TypeScript::side_module_init(rt).await?;

    inject_env_vars(rt)?;

    let ts = Main::main_module_init(rt).await?;

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ts/src/lib.ts");

    let js = ts.blank_space(std::fs::read_to_string(&source)?, rt)?;

    #[js(value(of_type(v8::Object)))]
    struct Module;

    let module: Module = {
        let url = Url::from_directory_path(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .join("ad-hoc.js")?;
        let id = rt.load_side_es_module_from_code(&url, js.clone()).await?;
        rt.mod_evaluate(id).await?;
        rt.get_module_namespace(id)?.into()
    };

    impl Compiler for Module {}

    PrettyPrinter::new()
        .input_from_bytes(js.as_bytes())
        .language("javascript")
        .theme("GitHub")
        .print()?;

    println!();

    let errors = module
        .create_program(vec![source.to_string_lossy().into()], rt)?
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
