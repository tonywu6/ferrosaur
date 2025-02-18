use deno_bindgen3::{global_this, module, newtype, properties};
use deno_runtime::deno_core;

#[global_this]
pub struct Global;

#[properties]
impl Global {
    #[property(cast(v8))]
    pub fn console(&self) -> Console {}
}

#[newtype]
pub struct Console;

#[properties]
impl Console {
    #[function]
    pub async fn log(&self, #[argument(spread, cast(v8))] items: &[v8::Global<v8::Value>]) {}
}

#[module("./main.js", fast)]
pub struct Main;

#[properties]
impl Main {
    #[property(cast(v8))]
    pub fn calc(&self) -> Calculator {}

    #[function(constructor, name = "Calculator", cast(v8))]
    pub fn calculator(&self) -> Calculator {}
}

#[newtype]
pub struct Calculator;

#[properties]
impl Calculator {
    #[property(with_setter)]
    pub fn value(&self) -> f64 {}

    #[function(cast(v8))]
    pub async fn add(&self, value: f64) -> Self {}

    #[function(cast(v8))]
    pub async fn sub(&self, value: f64) -> Self {}

    #[function(cast(v8))]
    pub async fn mul(&self, value: f64) -> Self {}

    #[function(cast(v8))]
    pub async fn div(&self, value: f64) -> Self {}
}
