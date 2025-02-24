use std::{rc::Rc, sync::Arc};

use anyhow::Result;
use deno_runtime::{
    deno_fs::InMemoryFs,
    worker::{MainWorker, WorkerOptions},
};
use tap::Pipe;

use crate::modules::Main;

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

    let mut worker = MainWorker::bootstrap_from_options(
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
        WorkerOptions {
            ..Default::default()
        },
    );

    let main = Main::new(&mut worker.js_runtime).await?;

    Ok((worker, main))
}
