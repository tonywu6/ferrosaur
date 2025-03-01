use std::path::PathBuf;

use anyhow::Result;
use tap::Pipe;

mod compile;
mod util;

use self::{compile::modules::Cwd, util::deno};

#[tokio::test]
async fn test_import_url() -> Result<()> {
    let rt = &mut deno().await?;

    let cargo_toml = Cwd::new(rt)
        .await?
        .cargo_manifest_dir(rt)?
        .pipe(PathBuf::from)
        .join("Cargo.toml");

    assert!(cargo_toml.exists());

    Ok(())
}

#[tokio::test]
async fn test_arbitrary_url() -> Result<()> {
    let rt = &mut deno().await?;

    let version = Cwd::new(rt).await?.pkg_version(rt).await?;

    assert_eq!(version, "0.1.0");

    Ok(())
}
