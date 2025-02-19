use deno_bindgen3::js;
use deno_runtime::deno_core;

#[js(global_this)]
pub struct Global;

#[js(properties)]
impl Global {
    #[js(prop(cast(v8)))]
    pub fn console(&self) -> Console {}
}

#[js(value)]
pub struct Console;

#[js(properties)]
impl Console {
    #[js(func)]
    pub async fn log(&self, #[js(arg(spread, cast(v8)))] items: &[v8::Global<v8::Value>]) {}
}

#[js(module("./main.js", fast))]
pub struct Main;

#[js(properties)]
impl Main {
    #[js(prop(cast(v8)))]
    pub fn calc(&self) -> Calculator {}

    #[js(new)]
    pub fn calculator(&self) -> Calculator {}
}

#[js(value)]
pub struct Calculator;

#[js(properties)]
impl Calculator {
    #[js(prop(with_setter))]
    pub fn value(&self) -> f64 {}

    #[js(func(cast(v8)))]
    pub async fn add(&self, value: f64) -> Self {}

    #[js(func(cast(v8)))]
    pub async fn sub(&self, value: f64) -> Self {}

    #[js(func(cast(v8)))]
    pub async fn mul(&self, value: f64) -> Self {}

    #[js(func(cast(v8)))]
    pub async fn div(&self, value: f64) -> Self {}
}
