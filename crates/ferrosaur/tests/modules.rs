use std::path::PathBuf;

use anyhow::Result;
use tap::Pipe;

mod fixture;

use fixture::{deno, items::modules::Cwd};

#[tokio::test]
async fn test_import_url() -> Result<()> {
    let rt = &mut deno()?;

    let cargo_toml = Cwd::main_module_init(rt)
        .await?
        .cargo_manifest_dir(rt)?
        .pipe(PathBuf::from)
        .join("Cargo.toml");

    assert!(cargo_toml.exists());

    Ok(())
}

#[tokio::test]
async fn test_arbitrary_url() -> Result<()> {
    let rt = &mut deno()?;

    let version = Cwd::main_module_init(rt).await?.pkg_version(rt).await?;

    assert_eq!(version, "0.1.0");

    Ok(())
}
