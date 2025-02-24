use darling::{
    ast::NestedMeta,
    util::{path_to_string, Flag},
    Error, FromMeta, Result,
};
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{
    parse_macro_input, punctuated::Punctuated, Lit, LitStr, Meta, Path, PathSegment, Token, Type,
    TypePath,
};
use tap::Pipe;

mod fast_string;
mod global_this;
mod iterator;
mod module;
mod properties;
mod util;
mod value;

use crate::util::{FatalErrors, FlagEnum, FlagLike, FlagName, TokenStreamResult, Unary};

#[proc_macro_attribute]
pub fn js(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    js_item(attr.into(), item.into()).or_error().into()
}

fn js_item(args: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (js, errors) = FlagLike::<JsItem>::parse_macro_attribute(args).or_fatal(errors)?;

    let (js, errors) = match js.0 {
        JsItem::Module(FlagLike(module)) => module::module(module, item),
        JsItem::GlobalThis(FlagLike(global_this)) => global_this::global_this(global_this, item),
        JsItem::Value(FlagLike(value)) => value::value(value, item),
        JsItem::Properties(FlagLike(properties)) => properties::properties(properties, item),
        JsItem::Iterator(FlagLike(iterator)) => iterator::iterator(iterator, item),
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
    Properties(FlagLike<Properties>),
    Iterator(FlagLike<Iterator_>),
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
    of: InnerType,
}

#[derive(Debug, Clone)]
struct InnerType(Box<Type>);

#[derive(Debug, Default, Clone, FromMeta)]
struct Properties;

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

impl FromMeta for InnerType {
    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        let mut errors = Error::accumulator();
        let item = match items.len() {
            1 => &items[0],
            0 => {
                errors.finish()?;
                return Err(Value::error("must specify a type"));
            }
            _ => {
                errors.push(Value::error("must specify exactly 1 type"));
                &items[0]
            }
        };
        let (item, errors) = match item {
            NestedMeta::Lit(..) => {
                Value::error("unexpected literal, expected a type path").pipe(Err)
            }
            NestedMeta::Meta(item) => match item {
                Meta::List(..) => {
                    Value::error("unexpected nested list, expected a type path").pipe(Err)
                }
                Meta::NameValue(..) => {
                    Value::error("unexpected assignment, expected a type path").pipe(Err)
                }
                Meta::Path(path) => TypePath {
                    path: path.clone(),
                    qself: None,
                }
                .pipe(Type::Path)
                .pipe(Ok),
            },
        }
        .map_err(|err| err.with_span(item))
        .or_fatal(errors)?;
        errors.finish()?;
        Ok(Self(Box::new(item)))
    }
}

impl Default for InnerType {
    fn default() -> Self {
        [format_ident!("v8"), format_ident!("Value")]
            .map(PathSegment::from)
            .pipe(Punctuated::<PathSegment, Token![::]>::from_iter)
            .pipe(|segments| Path {
                segments,
                leading_colon: None,
            })
            .pipe(|path| TypePath { path, qself: None })
            .pipe(Type::Path)
            .pipe(Box::new)
            .pipe(Self)
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
        Properties::PREFIX,
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

impl FlagName for Properties {
    const PREFIX: &'static str = "properties";

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
