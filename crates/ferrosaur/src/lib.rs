use darling::{
    ast::NestedMeta,
    util::{path_to_string, Flag},
    Error, FromMeta, Result,
};
use proc_macro2::TokenStream;
use syn::{parse_macro_input, Lit, LitStr, Meta};

mod callable;
mod fast_string;
mod global_this;
mod interface;
mod iterator;
mod module;
mod util;
mod value;

use crate::util::{
    flag::{FlagEnum, FlagError, FlagLike, FlagName},
    positional::Positional,
    property::WellKnown,
    string::StringLike,
    unary::Unary,
    v8::V8InnerType,
    FatalErrors, TokenStreamResult,
};

/// Macro for deriving `struct`s and `impl`s for use with `deno_core`.
/// Please see the [crate-level documentation](.).
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
            global_this::global_this(global_this, item).error_at::<JsItem, GlobalThis>()
        }
        JsItem::Interface(FlagLike(interface)) => {
            interface::interface(interface, item).error_at::<JsItem, Interface>()
        }
        JsItem::Callable(FlagLike(callable)) => {
            callable::callable(callable, item).error_at::<JsItem, Callable>()
        }
        JsItem::Iterator(FlagLike(iterator)) => {
            iterator::iterator(iterator, item).error_at::<JsItem, Iterator_>()
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
    Callable(FlagLike<Callable>),
    Iterator(FlagLike<Iterator_>),
}

#[derive(Debug, Clone, FromMeta)]
#[darling(rename_all = "snake_case")]
enum JsProp {
    Prop(FlagLike<Property>),
    Func(FlagLike<Function>),
    New(FlagLike<Constructor>),
    GetIndex(FlagLike<Getter>),
    SetIndex(FlagLike<Setter>),
}

#[derive(Debug, Clone, FromMeta)]
#[doc = include_str!("../../../docs/src/reference/module.md")]
struct Module(Positional<String, ModuleOptions>);

#[derive(Debug, Clone, FromMeta)]
struct ModuleOptions {
    #[darling(default)]
    url: ImportMetaUrl,
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

#[doc = include_str!("../../../docs/src/reference/global-this.md")]
#[derive(Debug, Default, Clone, FromMeta)]
struct GlobalThis;

#[doc = include_str!("../../../docs/src/reference/value.md")]
#[derive(Debug, Default, Clone, FromMeta)]
struct Value {
    #[darling(default)]
    of_type: Unary<V8InnerType>,
}

#[doc = include_str!("../../../docs/src/reference/interface.md")]
#[doc = include_str!("../../../docs/src/reference/_snippets/todo-list.md")]
#[derive(Debug, Default, Clone, FromMeta)]
struct Interface;

#[doc = include_str!("../../../docs/src/reference/callable.md")]
#[derive(Debug, Default, Clone, FromMeta)]
struct Callable;

#[doc = include_str!("../../../docs/src/reference/iterator.md")]
#[derive(Debug, Default, Clone, FromMeta)]
struct Iterator_;

type PropKeyString = StringLike<String>;

type PropKeySymbol = StringLike<WellKnown>;

#[doc = include_str!("../../../docs/src/reference/interface/prop.md")]
#[derive(Debug, Default, Clone, FromMeta)]
struct Property {
    name: Option<Unary<PropKeyString>>,
    #[darling(rename = "Symbol")]
    symbol: Option<Unary<PropKeySymbol>>,
    with_setter: Flag,
}

#[doc = include_str!("../../../docs/src/reference/interface/func.md")]
#[derive(Debug, Default, Clone, FromMeta)]
struct Function {
    name: Option<Unary<PropKeyString>>,
    #[darling(rename = "Symbol")]
    symbol: Option<Unary<PropKeySymbol>>,
}

#[doc = include_str!("../../../docs/src/reference/interface/new.md")]
#[derive(Debug, Default, Clone, FromMeta)]
struct Constructor {
    class: Option<Unary<PropKeyString>>,
}

#[doc = include_str!("../../../docs/src/reference/interface/get-set.md")]
#[derive(Debug, Default, Clone, FromMeta)]
struct Getter;

#[derive(Debug, Default, Clone, FromMeta)]
struct Setter;

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
        Iterator_::PREFIX,
    ];
}

impl FlagName for JsProp {
    const PREFIX: &'static str = "js";

    fn unit() -> Result<Self> {
        JsProp::from_word()
    }
}

impl FlagEnum for JsProp {
    const PREFIXES: &'static [&'static str] = &[
        Property::PREFIX,
        Function::PREFIX,
        Constructor::PREFIX,
        Getter::PREFIX,
        Setter::PREFIX,
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

impl FlagName for Callable {
    const PREFIX: &'static str = "callable";

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

impl FlagName for Property {
    const PREFIX: &'static str = "prop";

    fn unit() -> Result<Self> {
        Ok(Default::default())
    }
}

impl FlagName for Function {
    const PREFIX: &'static str = "func";

    fn unit() -> Result<Self> {
        Ok(Default::default())
    }
}

impl FlagName for Constructor {
    const PREFIX: &'static str = "new";

    fn unit() -> Result<Self> {
        Ok(Default::default())
    }
}

impl FlagName for Getter {
    const PREFIX: &'static str = "get_index";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}

impl FlagName for Setter {
    const PREFIX: &'static str = "set_index";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}

#[proc_macro]
pub fn unsafe_include_fast_string(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path = parse_macro_input!(args as LitStr);
    fast_string::unsafe_include_fast_string(path).into()
}
