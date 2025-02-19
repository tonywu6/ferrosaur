use darling::{ast::NestedMeta, util::Flag, Error, FromMeta, Result};
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{punctuated::Punctuated, Meta, Path, PathSegment, Token, Type, TypePath};
use tap::Pipe;

mod global_this;
mod module;
mod properties;
mod util;
mod value;

use crate::util::{FatalErrors, Feature, FeatureEnum, FeatureName, TokenStreamResult};

#[proc_macro_attribute]
pub fn js(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    js_item(attr.into(), item.into()).or_error().into()
}

fn js_item(args: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (js, errors) = Feature::<JsObject>::parse_macro_attribute(args).or_fatal(errors)?;

    let (js, errors) = match js.0 {
        JsObject::Module(Feature(module)) => module::module(module, item),
        JsObject::GlobalThis(Feature(global_this)) => global_this::global_this(global_this, item),
        JsObject::Value(Feature(value)) => value::value(value, item),
        JsObject::Properties(Feature(properties)) => properties::properties(properties, item),
    }
    .or_fatal(errors)?;

    errors.finish_with(js)
}

#[derive(Debug, Clone, FromMeta)]
#[darling(rename_all = "snake_case")]
enum JsObject {
    Module(Feature<Module>),
    GlobalThis(Feature<GlobalThis>),
    Value(Feature<Value>),
    Properties(Feature<Properties>),
}

#[derive(Debug, Clone)]
struct Module {
    import: String,
    options: ModuleOptions,
}

#[derive(Debug, Default, Clone, FromMeta)]
struct GlobalThis;

#[derive(Debug, Default, Clone, FromMeta)]
struct Value {
    serde: Flag,
    #[darling(default)]
    of: InnerType,
}

#[derive(Debug, Default, Clone, FromMeta)]
struct Properties;

#[derive(Debug, Clone, FromMeta)]
struct ModuleOptions {
    fast: Flag,
    side: Flag,
    #[darling(default)]
    url: ImportMetaUrl,
}

#[derive(Debug, Default, Clone, Copy, FromMeta)]
#[darling(rename_all = "lowercase")]
enum ImportMetaUrl {
    #[default]
    Preserve,
    Cwd,
}

#[derive(Debug, Clone)]
struct InnerType(Box<Type>);

impl FromMeta for Module {
    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        let (import, options) = items
            .split_first()
            .ok_or_else(|| Module::error("must specify the file path to import"))?;

        let mut errors = Error::accumulator();

        let import = errors.handle(String::from_nested_meta(import));
        let options = errors.handle(ModuleOptions::from_list(options));

        errors.finish()?;

        let import = import.unwrap();
        let options = options.unwrap();

        Ok(Self { import, options })
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

impl FeatureName for JsObject {
    const PREFIX: &str = "js";

    fn unit() -> Result<Self> {
        JsObject::from_word()
    }
}

impl FeatureEnum for JsObject {
    const PREFIXES: &[&str] = &[
        Module::PREFIX,
        GlobalThis::PREFIX,
        Value::PREFIX,
        Properties::PREFIX,
    ];
}

impl FeatureName for Module {
    const PREFIX: &str = "module";

    fn unit() -> Result<Self> {
        Module::from_word()
    }
}

impl FeatureName for GlobalThis {
    const PREFIX: &str = "global_this";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}

impl FeatureName for Value {
    const PREFIX: &str = "value";

    fn unit() -> Result<Self> {
        Ok(Default::default())
    }
}

impl FeatureName for Properties {
    const PREFIX: &str = "properties";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}
