use std::{rc::Rc, sync::Arc};

use anyhow::Result;
use deno_runtime::{
    deno_core::{v8, FromV8, ToV8},
    deno_fs::InMemoryFs,
    worker::{MainWorker, WorkerOptions},
};
use tap::Pipe;

use super::Main;

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

/// Backported from deno_core@0.339.0
///
/// A wrapper type for `Option<T>` that (de)serializes `None` as `null`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct OptionNull<T>(pub Option<T>);

impl<T> From<Option<T>> for OptionNull<T> {
    fn from(option: Option<T>) -> Self {
        Self(option)
    }
}

impl<T> From<OptionNull<T>> for Option<T> {
    fn from(value: OptionNull<T>) -> Self {
        value.0
    }
}

impl<'a, T> ToV8<'a> for OptionNull<T>
where
    T: ToV8<'a>,
{
    type Error = T::Error;

    fn to_v8(
        self,
        scope: &mut v8::HandleScope<'a>,
    ) -> Result<v8::Local<'a, v8::Value>, Self::Error> {
        match self.0 {
            Some(value) => value.to_v8(scope),
            None => Ok(v8::null(scope).into()),
        }
    }
}

impl<'a, T> FromV8<'a> for OptionNull<T>
where
    T: FromV8<'a>,
{
    type Error = T::Error;

    fn from_v8(
        scope: &mut v8::HandleScope<'a>,
        value: v8::Local<'a, v8::Value>,
    ) -> Result<Self, Self::Error> {
        if value.is_null() {
            Ok(OptionNull(None))
        } else {
            T::from_v8(scope, value).map(|v| OptionNull(Some(v)))
        }
    }
}
