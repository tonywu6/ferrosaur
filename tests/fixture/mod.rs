use deno_bindgen3::js;
use deno_core::convert::OptionNull;

pub mod deno;

#[js(global_this)]
pub struct Global;

#[js(properties)]
impl Global {
    #[js(prop)]
    pub fn console(&self) -> Console {}
}

#[js(value)]
struct Console;

#[js(properties)]
impl Console {
    #[js(func(name(log)))]
    pub fn log(&self, ..values: &[v8::Global<v8::Value>]) {}

    #[js(func(name(log)))]
    pub fn log_message(&self, message: serde<&str>) {}
}

#[js(module(import("js/mod.js"), fast))]
pub struct Main;

#[js(properties)]
impl Main {
    #[js(new)]
    pub fn rectangle(&self, w: serde<f64>, h: serde<f64>) -> Rectangle {}

    #[js(new(class(ThisConsideredHarmful)))]
    pub fn this_checker(&self) -> ThisChecker {}

    #[js(func)]
    pub async fn sleep(&self, value: bool, ms: serde<usize>) -> bool {}
}

#[js(value)]
pub struct Rectangle;

#[js(properties)]
impl Rectangle {
    #[js(prop(with_setter))]
    pub fn height(&self) -> serde<f64> {}

    #[js(prop(with_setter))]
    pub fn width(&self) -> serde<f64> {}

    #[js(func)]
    pub fn area(&self) -> serde<f64> {}

    #[js(func(Symbol(toPrimitive)))]
    pub fn value(&self) -> serde<serde_json::Value> {}

    #[js(func(name = "maybeSquare"))]
    pub fn square(&self) -> OptionNull<Square> {}
}

#[js(value)]
struct Square;

#[js(value)]
struct ThisChecker;

#[js(properties)]
impl ThisChecker {
    #[js(func(name(whoami), this(self)))]
    pub fn get_this(&self) -> v8::Global<v8::Value> {}

    #[js(func(name(whoami), this(undefined)))]
    pub fn get_undefined(&self) -> v8::Global<v8::Value> {}
}

#[js(module(import("js/i18n.js"), side_module))]
pub struct I18n;

#[js(properties)]
impl I18n {
    #[js(prop(name = "the quick brown fox jumps over the lazy dog"))]
    pub fn en_us(&self) -> serde<String> {}

    #[js(prop(name = "天地玄黄，宇宙洪荒"))]
    pub fn zh_cn(&self) -> serde<String> {}
}

#[js(module(import("js/iter.js"), fast, side_module))]
pub struct Iter;

#[js(properties)]
impl Iter {
    #[js(func)]
    pub fn fibonacci(&self, iter: serde<usize>) -> Fibonacci {}
}

#[js(value)]
struct Fibonacci;

#[js(iterator)]
impl Fibonacci {
    type Item = serde<usize>;
}
