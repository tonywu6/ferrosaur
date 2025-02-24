use deno_bindgen3::js;
use deno_runtime::deno_core;

pub mod global;
pub mod i18n;
pub mod iter;

#[js(module(import("mod.js"), fast))]
pub struct Main;

#[js(properties)]
impl Main {
    #[js(new)]
    pub fn rectangle(&self, w: f64, h: f64) -> v8<Rectangle> {}

    #[js(new(class(ThisConsideredHarmful)))]
    pub fn this_checker(&self) -> v8<ThisChecker> {}
}

#[js(value)]
pub struct Rectangle;

#[js(properties)]
impl Rectangle {
    #[js(prop(with_setter))]
    pub fn height(&self) -> f64 {}

    #[js(prop(with_setter))]
    pub fn width(&self) -> f64 {}

    #[js(func)]
    pub fn area(&self) -> f64 {}

    #[js(func(Symbol(toPrimitive)))]
    pub fn value(&self) -> serde_json::Value {}

    #[js(func(name = "maybeSquare"))]
    pub fn square(&self) -> Option<v8<Square>> {}
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
