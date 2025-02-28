use deno_core::convert::OptionNull;
use ferrosaur::js;

#[js(global_this)]
pub struct Global;

#[js(interface)]
impl Global {
    #[js(prop)]
    pub fn console(&self) -> Console {}

    #[js(set)]
    pub fn declare(&self, name: serde<&str>, value: v8::Global<v8::Value>) {}
}

#[js(interface)]
impl Global {
    #[js(func(name(__cargo_test_stdout__)))]
    pub fn cargo_test_stdout(&self) -> String {}
}

#[js(value)]
struct Console;

#[js(interface)]
impl Console {
    #[js(func(name(log)))]
    pub fn log(&self, ..values: &[v8::Global<v8::Value>]) {}

    #[js(func(name(log)))]
    pub fn log_message(&self, message: serde<&str>) {}
}

#[js(module(import("js/mod.js"), fast))]
pub struct Main;

#[js(interface)]
impl Main {
    #[js(func)]
    pub async fn sleep(&self, value: bool, ms: serde<usize>) -> bool {}

    #[js(func)]
    pub fn use_navigate(&self) -> NavigateFn {}

    #[js(new)]
    pub fn rectangle(&self, w: serde<f64>, h: serde<f64>) -> Rectangle {}

    #[js(new(class(ThisConsideredHarmful)))]
    pub fn this_checker(&self) -> ThisChecker {}
}

#[js(value)]
pub struct Rectangle;

#[js(interface)]
impl Rectangle {
    #[js(prop)]
    pub fn height(&self) -> serde<f64> {}

    #[js(prop(with_setter))]
    pub fn width(&self) -> serde<f64> {}

    #[js(func)]
    pub fn area(&self) -> serde<f64> {}

    #[js(func(Symbol(toPrimitive)))]
    pub fn value(&self) -> serde<serde_json::Value> {}

    #[js(func(name = "maybeSquare"))]
    pub fn square(&self) -> OptionNull<Rectangle> {}
}

#[js(value)]
struct ThisChecker;

#[js(interface)]
impl ThisChecker {
    #[js(func(name(whoami)))]
    pub fn get_this(&self) -> v8::Global<v8::Value> {}

    #[js(func(name(whoami)))]
    pub fn get_undefined(&self, this: undefined) -> v8::Global<v8::Value> {}

    #[js(func(name(whoami)))]
    pub fn get_unbound(&self, this: v8::Global<v8::Value>) -> v8::Global<v8::Value> {}
}

#[js(value(of_type(v8::Function)))]
struct NavigateFn;

#[js(function)]
impl NavigateFn {
    pub fn call(&self, path: serde<&str>) {}
}

#[js(module(import("js/i18n.js"), side_module))]
pub struct I18n;

#[js(interface)]
impl I18n {
    #[js(prop(name = "The quick brown fox jumps over the lazy dog"))]
    pub fn en_us(&self) -> serde<String> {}

    #[js(prop(name = "天地玄黄，宇宙洪荒"))]
    pub fn zh_cn(&self) -> serde<String> {}

    #[js(get)]
    pub fn i18n(&self, key: serde<&str>) -> serde<String> {}
}

#[js(module(import("js/iter.js"), fast, side_module))]
pub struct Iter;

#[js(interface)]
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
