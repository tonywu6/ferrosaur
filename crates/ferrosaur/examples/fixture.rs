use anyhow::Result;
use deno_core::StaticModuleLoader;

pub mod modules {
    use ferrosaur::js;

    #[js(module("js/mod.js", fast))]
    pub struct Main;

    #[js(module("js/i18n.js"))]
    pub struct I18n;

    #[js(module("js/iter.js", fast))]
    pub struct Iter;

    #[js(module("js/cwd.js", url(cwd), fast))]
    pub struct Cwd;

    #[js(module("js/pkg.js", url("cargo:tests/js/pkg"), fast(unsafe_debug)))]
    pub struct Pkg;
}

pub mod global {
    use ferrosaur::js;

    #[js(global_this)]
    pub struct Global;
}

pub mod values {
    use ferrosaur::js;

    #[js(value)]
    pub struct Console;

    #[js(value)]
    pub struct Rectangle;

    #[js(value(of_type(v8::Object)))]
    pub(super) struct ThisChecker;

    #[js(value(of_type(v8::Function)))]
    pub(super) struct NavigateFn;

    #[js(value)]
    pub struct Fibonacci;

    #[js(value(of_type(v8::Map)))]
    pub struct Messages;

    #[js(value)]
    pub struct MessageIter;
}

pub mod props {
    use ferrosaur::js;

    use super::{
        global::Global,
        modules::I18n,
        values::{Console, Rectangle},
    };

    #[js(interface)]
    impl Rectangle {
        #[js(prop(with_setter))]
        pub fn width(&self) -> serde<f64> {}

        #[js(prop)]
        pub fn height(&self) -> serde<f64> {}
    }

    #[js(interface)]
    impl I18n {
        #[js(prop(name = "The quick brown fox jumps over the lazy dog"))]
        pub fn en_us(&self) -> serde<String> {}

        #[js(prop(name = "天地玄黄，宇宙洪荒"))]
        pub fn zh_cn(&self) -> serde<String> {}
    }

    #[js(interface)]
    impl Global {
        #[js(prop)]
        pub fn console(&self) -> Console {}
    }
}

pub mod funcs {
    use deno_core::convert::OptionNull;
    use ferrosaur::js;

    use super::{
        global::Global,
        modules::{Cwd, I18n, Iter, Main},
        values::{Console, Fibonacci, MessageIter, Messages, NavigateFn, Rectangle, ThisChecker},
    };

    #[js(interface)]
    impl Main {
        #[js(func)]
        pub async fn sleep(&self, value: bool, ms: serde<usize>) -> bool {}

        #[js(func)]
        pub fn use_navigate(&self) -> NavigateFn {}
    }

    #[js(interface)]
    impl Rectangle {
        #[js(func(name = "maybeSquare"))]
        pub fn square(&self) -> OptionNull<Rectangle> {}
    }

    #[js(interface)]
    impl ThisChecker {
        #[js(func(name(whoami)))]
        pub fn get_this(&self) -> v8::Global<v8::Value> {}

        #[js(func(name(whoami)))]
        pub fn get_undefined(&self, this: undefined) -> v8::Global<v8::Value> {}

        #[js(func(name(whoami)))]
        pub fn get_unbound(&self, this: v8::Global<v8::Value>) -> v8::Global<v8::Value> {}
    }

    #[js(interface)]
    impl Iter {
        #[js(func)]
        pub fn fibonacci(&self, iter: serde<usize>) -> Fibonacci {}
    }

    #[js(interface)]
    impl Cwd {
        #[js(func)]
        pub fn cargo_manifest_dir(&self) -> String {}

        #[js(func)]
        pub async fn pkg_version(&self) -> String {}
    }

    #[js(interface)]
    impl Console {
        #[js(func(name(log)))]
        pub fn log(&self, ..values: &[v8::Global<v8::Value>]) {}

        #[js(func(name(log)))]
        pub fn log_message(&self, message: serde<&str>) {}
    }

    #[js(interface)]
    impl Global {
        #[js(func(name(Boolean)))]
        pub fn boolean(&self, v: serde<bool>) -> v8::Global<v8::Value> {}

        #[js(func(name(Number)))]
        pub fn number(&self, v: serde<f64>) -> v8::Global<v8::Value> {}

        #[js(func(name(String)))]
        pub fn string(&self, v: serde<&str>) -> v8::Global<v8::Value> {}

        #[js(new(class(Date)))]
        pub fn date(&self, v: serde<f64>) -> v8::Global<v8::Value> {}
    }

    #[js(interface)]
    impl Global {
        #[js(prop(name(__cargo_test_stdout__)))]
        pub fn cargo_test_stdout(&self) -> String {}
    }

    #[js(interface)]
    impl I18n {
        #[js(func)]
        pub async fn messages(&self) -> Messages {}
    }

    #[js(interface)]
    impl Messages {
        #[js(func(Symbol(iterator)))]
        pub fn entries(&self) -> MessageIter {}
    }
}

pub mod ctors {
    use ferrosaur::js;

    use super::{
        modules::Main,
        values::{Rectangle, ThisChecker},
    };

    #[js(interface)]
    impl Main {
        #[js(new)]
        pub fn rectangle(&self, w: serde<f64>, h: serde<f64>) -> Rectangle {}

        #[js(new(class(ThisConsideredHarmful)))]
        pub fn this_checker(&self) -> ThisChecker {}
    }
}

pub mod protocols {
    use ferrosaur::js;

    use super::values::{Fibonacci, NavigateFn};

    #[js(callable)]
    impl NavigateFn {
        pub fn call(&self, path: serde<&str>) {}
    }

    #[js(iterator)]
    impl Fibonacci {
        type Item = serde<usize>;
    }
}

pub mod traits {
    use ferrosaur::js;
    use serde::de::DeserializeOwned;

    use super::values::{MessageIter, Rectangle};

    #[js(interface)]
    pub trait Shape {
        #[js(func)]
        fn area(&self) -> serde<f64>;

        #[js(func(Symbol(toPrimitive)))]
        fn value(&self) -> serde<serde_json::Value>;
    }

    impl Shape for Rectangle {}

    #[js(iterator)]
    pub trait Entries<K, V>: Sized
    where
        K: DeserializeOwned,
        V: DeserializeOwned,
    {
        type Item = serde<(K, V)>;
    }

    impl Entries<String, String> for MessageIter {}
}

pub mod indexing {
    use ferrosaur::js;

    use super::{global::Global, modules::I18n};

    #[js(interface)]
    impl I18n {
        #[js(get_index)]
        pub fn i18n(&self, key: serde<&str>) -> serde<String> {}
    }

    #[js(interface)]
    impl Global {
        #[js(get_index)]
        pub fn lookup<T: for<'a> FromV8<'a>>(&self, name: serde<&str>) -> T {}

        #[js(set_index)]
        pub fn define(&self, name: serde<&str>, value: v8::Global<v8::Value>) {}
    }
}

pub fn modules() -> Result<StaticModuleLoader> {
    use self::modules::{Cwd, I18n, Iter, Main, Pkg};

    Ok(StaticModuleLoader::new([
        Main::preloaded()?,
        Iter::preloaded()?,
        I18n::preloaded()?,
        Cwd::preloaded()?,
        Pkg::preloaded()?,
    ]))
}

#[allow(unused)]
fn main() {}
