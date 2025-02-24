use deno_bindgen3::js;
use deno_runtime::deno_core;

#[js(global_this)]
pub struct Global;

#[js(properties)]
impl Global {
    #[js(prop)]
    pub fn console(&self) -> v8<Console> {}
}

#[js(value)]
struct Console;

#[js(properties)]
impl Console {
    #[js(func(name(log)))]
    pub fn log(&self, ..values: &[v8::Global<v8::Value>]) {}

    #[js(func(name(log)))]
    pub fn log_message(&self, message: &str) {}
}
