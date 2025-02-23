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
pub struct Console;

#[js(properties)]
impl Console {
    #[js(func)]
    pub fn log(&self, ..msgs: Vec<String>, ..items: &[v8::Global<v8::Value>]) {}
}

#[js(module(import("./main.js"), fast))]
pub struct Main;

#[js(properties)]
impl Main {
    #[js(prop)]
    pub fn calc(&self) -> v8<Calculator> {}

    #[js(new)]
    pub fn calculator(&self) -> v8<Calculator> {}

    #[js(new)]
    pub fn fibonacci(&self) -> v8<Fibonacci> {}
}

#[js(value)]
#[derive(Debug, Clone)]
pub struct Calculator;

#[js(properties)]
impl Calculator {
    #[js(prop)]
    pub fn value(&self) -> f64 {}

    #[js(func)]
    pub fn add(&self, value: f64) -> v8<Self> {}

    #[js(func)]
    pub fn sub(&self, value: f64) -> v8<Self> {}

    #[js(func)]
    pub fn mul(&self, value: f64) -> v8<Self> {}

    #[js(func)]
    pub fn div(&self, value: f64) -> v8<Self> {}

    #[js(prop(Symbol(toStringTag)))]
    pub fn to_string(&self) -> String {}
}

#[js(value)]
pub struct Fibonacci;

#[js(properties)]
impl Fibonacci {
    #[js(func(Symbol(iterator)))]
    pub fn iter(&self) -> v8<FibonacciIter> {}
}

#[js(value)]
pub struct FibonacciIter;

#[js(iterator)]
impl FibonacciIter {
    type Item = f64;
}
