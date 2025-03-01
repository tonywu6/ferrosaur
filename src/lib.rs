use darling::{
    ast::NestedMeta,
    util::{path_to_string, Flag},
    Error, FromMeta, Result,
};
use proc_macro2::TokenStream;
use syn::{parse_macro_input, Lit, LitStr, Meta};
use tap::Pipe;

mod fast_string;
mod function;
mod global;
mod interface;
mod iterator;
mod module;
mod util;
mod value;

use crate::util::{
    flag::{FlagEnum, FlagLike, FlagName},
    unary::Unary,
    v8::V8InnerType,
    ErrorLocation, FatalErrors, TokenStreamResult,
};

#[proc_macro_attribute]
pub fn js(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    js_item(attr.into(), item.into()).or_error().into()
}

fn js_item(args: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (js, errors) = FlagLike::<JsItem>::parse_macro_attribute(args).or_fatal(errors)?;

    let (js, errors) = match js.0 {
        JsItem::Value(FlagLike(value)) => value::value(value, item).error_at::<JsItem, Value>(),

        JsItem::Module(FlagLike(module)) => {
            module::module(module, item).error_at::<JsItem, Module>()
        }
        JsItem::GlobalThis(FlagLike(global_this)) => {
            global::global_this(global_this, item).error_at::<JsItem, GlobalThis>()
        }
        JsItem::Interface(FlagLike(interface)) => {
            interface::interface(interface, item).error_at::<JsItem, Interface>()
        }
        JsItem::Function(FlagLike(function)) => {
            function::function(function, item).error_at::<JsItem, Function>()
        }
        JsItem::Iterator(FlagLike(iterator)) => {
            iterator::iterator(iterator, item).error_at::<JsItem, Iterator_>()
        }
        JsItem::Prop | JsItem::Func | JsItem::New | JsItem::Get | JsItem::Set => {
            "should be use within a #[js(interface)] impl or trait"
                .pipe(Error::custom)
                .pipe(Err)
        }
    }
    .or_fatal(errors)?;

    errors.finish_with(js)
}

#[derive(Debug, Clone, FromMeta)]
#[darling(rename_all = "snake_case")]
enum JsItem {
    Module(FlagLike<Module>),
    GlobalThis(FlagLike<GlobalThis>),
    Value(FlagLike<Value>),
    Interface(FlagLike<Interface>),
    Function(FlagLike<Function>),
    Iterator(FlagLike<Iterator_>),
    Prop,
    Func,
    New,
    Get,
    Set,
}

#[derive(Debug, Clone, FromMeta)]
struct Module {
    import: Unary<String>,
    #[darling(default)]
    url: ImportMetaUrl,
    side_module: Flag,
    fast: Option<FastString>,
}

#[derive(Debug, Default, Clone)]
enum ImportMetaUrl {
    #[default]
    Preserve,
    Cwd,
    Url(String),
}

#[derive(Debug, Clone, Copy)]
enum FastString {
    Fast,
    FastUnsafeDebug,
}

#[derive(Debug, Default, Clone, FromMeta)]
struct GlobalThis;

#[derive(Debug, Default, Clone, FromMeta)]
struct Value {
    #[darling(default)]
    of_type: Unary<V8InnerType>,
}

#[derive(Debug, Default, Clone, FromMeta)]
struct Interface;

#[derive(Debug, Default, Clone, FromMeta)]
struct Function;

#[derive(Debug, Default, Clone, FromMeta)]
struct Iterator_;

impl FromMeta for ImportMetaUrl {
    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        match items.len() {
            0 => Err(Error::too_few_items(1)),
            1 => match &items[0] {
                NestedMeta::Meta(meta) => match path_to_string(meta.path()).as_str() {
                    "preserve" => Ok(Self::Preserve),
                    "cwd" => Ok(Self::Cwd),
                    other => Err(Error::unknown_field_with_alts(
                        other,
                        &["preserve", "cwd", "a string literal"],
                    )),
                },
                NestedMeta::Lit(Lit::Str(lit)) => Ok(Self::Url(lit.value())),
                NestedMeta::Lit(lit) => Err(Error::unexpected_lit_type(lit)),
            },
            _ => Err(Error::too_many_items(1)),
        }
    }
}

impl FromMeta for FastString {
    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        match items.len() {
            0 => Self::from_word(),
            1 => {
                let path = match &items[0] {
                    NestedMeta::Meta(Meta::Path(path)) => path_to_string(path),
                    NestedMeta::Lit(Lit::Str(lit)) => lit.value(),
                    NestedMeta::Meta(Meta::List(_)) => {
                        return Err(Error::unsupported_format("list"))
                    }
                    NestedMeta::Meta(Meta::NameValue(_)) => {
                        return Err(Error::unsupported_format("name value"))
                    }
                    NestedMeta::Lit(lit) => return Err(Error::unexpected_lit_type(lit)),
                };
                match &*path {
                    "unsafe_debug" => Ok(Self::FastUnsafeDebug),
                    path => Err(Error::unknown_field_with_alts(path, &["unsafe_debug"])),
                }
            }
            _ => Err(Error::too_many_items(1)),
        }
    }

    fn from_word() -> Result<Self> {
        Ok(Self::Fast)
    }
}

impl FlagName for JsItem {
    const PREFIX: &'static str = "js";

    fn unit() -> Result<Self> {
        JsItem::from_word()
    }
}

impl FlagEnum for JsItem {
    const PREFIXES: &'static [&'static str] = &[
        Module::PREFIX,
        GlobalThis::PREFIX,
        Value::PREFIX,
        Interface::PREFIX,
    ];
}

impl FlagName for Module {
    const PREFIX: &'static str = "module";

    fn unit() -> Result<Self> {
        Module::from_word()
    }
}

impl FlagName for GlobalThis {
    const PREFIX: &'static str = "global_this";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}

impl FlagName for Value {
    const PREFIX: &'static str = "value";

    fn unit() -> Result<Self> {
        Ok(Default::default())
    }
}

impl FlagName for Interface {
    const PREFIX: &'static str = "interface";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}

impl FlagName for Function {
    const PREFIX: &'static str = "function";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}

impl FlagName for Iterator_ {
    const PREFIX: &'static str = "iterator";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}

#[proc_macro]
pub fn unsafe_include_fast_string(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path = parse_macro_input!(args as LitStr);
    fast_string::unsafe_include_fast_string(path).into()
}
