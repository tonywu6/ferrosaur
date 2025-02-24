use deno_bindgen3::js;
use deno_runtime::deno_core;

#[js(module(import("./iter.js"), fast, side_module))]
pub struct Iter;

#[js(properties)]
impl Iter {
    #[js(func)]
    pub fn fibonacci(&self, iter: usize) -> v8<Fibonacci> {}
}

#[js(value)]
struct Fibonacci;

#[js(iterator)]
impl Fibonacci {
    type Item = usize;
}
