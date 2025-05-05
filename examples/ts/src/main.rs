use std::{collections::HashMap, path::Path};

use anyhow::Result;

use example_runtime::deno;
use example_ts::{inject_env_vars, Compiler, Example, TypeScriptLib, TypeScriptVfs};
use tap::{Pipe, Tap};

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut deno()?;

    inject_env_vars(rt)?;

    TypeScriptLib::side_module_init(rt).await?;

    TypeScriptVfs::side_module_init(rt).await?;

    let example = Example::main_module_init(rt).await?;

    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/lib.ts")
        .pipe(std::fs::read_to_string)?;

    let root = HashMap::new().tap_mut(|map| drop(map.insert("src/lib.ts".into(), file)));

    let program = example.create_program(root, rt)?;

    let errors = program.print_diagnostics(true, rt)?;

    println!("{errors}");

    let errors = program.print_diagnostics(false, rt)?;

    insta::assert_snapshot!(errors, @r"
    src/lib.ts(1,16): error TS2307: Cannot find module 'npm:typescript' or its corresponding type declarations.
    src/lib.ts(6,8): error TS2307: Cannot find module 'npm:@typescript/vfs' or its corresponding type declarations.
    ");

    Ok(())
}
