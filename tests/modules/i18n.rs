use deno_bindgen3::js;
use deno_runtime::deno_core;

#[js(module(import("./i18n.js"), side_module))]
pub struct I18n;

#[js(properties)]
impl I18n {
    #[js(prop(name = "the quick brown fox jumps over the lazy dog"))]
    pub fn en_us(&self) -> String {}

    #[js(prop(name = "天地玄黄，宇宙洪荒"))]
    pub fn zh_cn(&self) -> String {}
}
