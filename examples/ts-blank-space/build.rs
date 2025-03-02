use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

fn main() -> Result<()> {
    println!("cargo::rerun-if-changed=build.js");

    let built = Command::new("deno")
        .args(["run", "--allow-all", "build.js"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to bundle JavaScript resources, is Deno installed?")?
        .wait()?
        .success();

    if !built {
        bail!("failed to bundle JavaScript resources")
    }

    Ok(())
}
