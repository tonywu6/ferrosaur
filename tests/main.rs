use std::{rc::Rc, sync::Arc};

use anyhow::Result;
use deno_runtime::{deno_fs::InMemoryFs, worker::MainWorker};
use tap::Pipe;

mod usage;

use self::usage::{Global, Main};

pub async fn deno() -> Result<(MainWorker, Main)> {
    use deno_runtime::{
        deno_core::StaticModuleLoader,
        deno_permissions::{Permissions, PermissionsContainer},
        permissions::RuntimePermissionDescriptorParser,
        worker::{MainWorker, WorkerServiceOptions},
    };

    let fs = Arc::new(InMemoryFs::default());

    let module_loader = Rc::new(StaticModuleLoader::default());

    let permissions = RuntimePermissionDescriptorParser::new(fs.clone())
        .pipe(|p| PermissionsContainer::new(Arc::new(p), Permissions::none_without_prompt()));

    let mut rt = MainWorker::bootstrap_from_options(
        Main::url()?,
        WorkerServiceOptions {
            fs,
            module_loader,
            permissions,
            blob_store: Default::default(),
            broadcast_channel: Default::default(),
            feature_checker: Default::default(),
            node_services: Default::default(),
            npm_process_state_provider: Default::default(),
            root_cert_store_provider: Default::default(),
            fetch_dns_resolver: Default::default(),
            shared_array_buffer_store: Default::default(),
            compiled_wasm_module_store: Default::default(),
            v8_code_cache: Default::default(),
        },
        Default::default(),
    );

    let main = Main::new(&mut rt.js_runtime).await?;

    Ok((rt, main))
}

#[tokio::test]
async fn test_calc() -> Result<()> {
    let (mut rt, main) = deno().await?;

    let rt = &mut rt.js_runtime;

    let calc = main.calc(rt)?;

    let calc = calc
        .add(16.0, rt)?
        .sub(4.0, rt)?
        .mul(7.0, rt)?
        .div(2.0, rt)?;

    assert_eq!(calc.value(rt)?, 42.0);

    Global::new(rt)
        .console(rt)?
        .log(&[calc.clone().into()], rt)?;

    println!("{}", calc.to_string(rt)?);

    Ok(())
}
