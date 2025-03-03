use std::path::Path;

use anyhow::Result;

use example_runtime::deno;
use example_ts::{Compiler, Example, TypeScript, inject_env_vars};

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut deno(Example::module_url()?)?.js_runtime;

    inject_env_vars(rt)?;

    TypeScript::side_module(rt).await?;

    let ts = Example::main_module(rt).await?;

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/lib.ts")
        .to_string_lossy()
        .into_owned();

    let program = ts.create_program(vec![path], rt)?;

    let errors = program.print_diagnostics(true, rt)?;

    println!("{errors}");

    let errors = program.print_diagnostics(false, rt)?;

    insta::assert_snapshot!(errors, @r###"
    src/lib.ts(1,16): error TS2307: Cannot find module 'npm:typescript' or its corresponding type declarations.
    src/lib.ts(41,20): error TS7006: Parameter 'fileName' implicitly has an 'any' type.
    src/lib.ts(65,63): error TS2304: Cannot find name 'Deno'.
    src/lib.ts(66,26): error TS7006: Parameter 'name' implicitly has an 'any' type.
    "###);

    Ok(())
}
