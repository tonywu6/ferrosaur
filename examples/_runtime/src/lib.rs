use std::{rc::Rc, sync::Arc};

use anyhow::Result;
pub use deno_runtime::deno_core::{self, serde_v8, v8};
use deno_runtime::{
    deno_core::{ModuleSpecifier, StaticModuleLoader},
    deno_fs::RealFs,
    deno_permissions::PermissionsContainer,
    permissions::RuntimePermissionDescriptorParser,
    worker::{MainWorker, WorkerOptions, WorkerServiceOptions},
};

pub fn deno(url: ModuleSpecifier) -> Result<MainWorker> {
    Ok(MainWorker::bootstrap_from_options(
        url,
        WorkerServiceOptions {
            blob_store: Default::default(),
            broadcast_channel: Default::default(),
            feature_checker: Default::default(),
            fs: Arc::new(RealFs),
            module_loader: Rc::new(StaticModuleLoader::default()),
            node_services: Default::default(),
            npm_process_state_provider: Default::default(),
            permissions: PermissionsContainer::allow_all(Arc::new(
                RuntimePermissionDescriptorParser::new(Arc::new(RealFs)),
            )),
            root_cert_store_provider: Default::default(),
            fetch_dns_resolver: Default::default(),
            shared_array_buffer_store: Default::default(),
            compiled_wasm_module_store: Default::default(),
            v8_code_cache: Default::default(),
        },
        WorkerOptions {
            ..Default::default()
        },
    ))
}
