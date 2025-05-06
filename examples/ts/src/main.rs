#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut deno()?;

    // Initialize the modules:

    use example_ts::{inject_lib_dts, Example, TypeScriptLib, TypeScriptVfs};

    TypeScriptLib::side_module_init(rt).await?;
    TypeScriptVfs::side_module_init(rt).await?;

    let example = Example::main_module_init(rt).await?;

    // Prepare the VFS. This is very similar to the [example in `@typescript/vfs`][vfs-example]:

    inject_lib_dts(rt)?;

    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/lib.ts")
        .pipe(std::fs::read_to_string)?;

    let root = {
        let mut map = HashMap::new();
        map.insert("src/lib.ts".into(), file);
        map
    };

    // Now, type check:

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

// <details>
//   <summary>Additional setup code</summary>

use std::{collections::HashMap, path::Path};

use anyhow::Result;

use example_runtime::deno;
use example_ts::Compiler;
use tap::Pipe;

// </details>

// [vfs-example]: https://www.npmjs.com/package/@typescript/vfs#a-full-example
