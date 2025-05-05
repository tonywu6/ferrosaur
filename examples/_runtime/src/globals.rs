use anyhow::Context;
use deno_core::{op2, ToJsBuffer};
use tap::{Conv, Pipe};

#[op2]
#[serde]
pub fn op_example_read_file(#[string] path: String) -> Result<ToJsBuffer, JsError> {
    std::fs::read(&path)
        .context(path)
        .context("failed to read file")
        .or_error()?
        .conv::<ToJsBuffer>()
        .pipe(Ok)
}

#[op2]
#[string]
pub fn op_example_cwd() -> Result<String, JsError> {
    Ok(std::env::current_dir().or_error()?.display().to_string())
}

#[derive(Debug, deno_error::JsError)]
#[class(generic)]
struct JsError(anyhow::Error);

impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.0))
    }
}

impl std::error::Error for JsError {}

trait IntoJsError<T> {
    fn or_error(self) -> Result<T, JsError>;
}

impl<T, E: Into<anyhow::Error>> IntoJsError<T> for Result<T, E> {
    fn or_error(self) -> Result<T, JsError> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(JsError(error.into())),
        }
    }
}
