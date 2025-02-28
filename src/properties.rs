use darling::{util::Flag, Error, FromMeta, Result};
use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, Parser},
    spanned::Spanned,
    Ident, ImplItemFn, ImplItemType, TraitItemFn, TraitItemType, Visibility,
};
use tap::Pipe;

use crate::{
    util::{
        no_default_fn, no_fn_body, Caveat, DeriveInterface, ErrorLocation, FatalErrors, FlagEnum,
        FlagLike, FlagName, InterfaceLike, OuterType, PropertyKey, SomeFunc, SomeType, StringLike,
        Unary, WellKnown,
    },
    Properties,
};

mod func;
mod get;
mod prop;
mod set;

#[derive(Debug, Clone, FromMeta)]
enum JsProperty {
    Prop(FlagLike<Property>),
    Func(FlagLike<Function>),
    New(FlagLike<Constructor>),
    Get(FlagLike<Getter>),
    Set(FlagLike<Setter>),
}

type PropKeyString = StringLike<String>;

type PropKeySymbol = StringLike<WellKnown>;

#[derive(Debug, Default, Clone, FromMeta)]
struct Property {
    name: Option<Unary<PropKeyString>>,
    #[darling(rename = "Symbol")]
    symbol: Option<Unary<PropKeySymbol>>,
    with_setter: Flag,
}

#[derive(Debug, Default, Clone, FromMeta)]
struct Function {
    name: Option<Unary<PropKeyString>>,
    #[darling(rename = "Symbol")]
    symbol: Option<Unary<PropKeySymbol>>,
}

#[derive(Debug, Default, Clone, FromMeta)]
struct Getter;

#[derive(Debug, Default, Clone, FromMeta)]
struct Setter;

#[derive(Debug, Default, Clone, FromMeta)]
struct Constructor {
    class: Option<Unary<PropKeyString>>,
}

pub fn properties(_: Properties, item: TokenStream) -> Result<TokenStream> {
    InterfaceLike::parse
        .parse2(item)?
        .derive::<DeriveProperties>()
}

struct DeriveProperties;

impl DeriveInterface for DeriveProperties {
    fn impl_func(item: ImplItemFn) -> Result<SomeFunc> {
        let ImplItemFn {
            attrs,
            vis,
            defaultness,
            sig,
            block,
        } = item;

        let mut errors = Error::accumulator();

        errors.handle(no_fn_body(Some(block)));
        errors.handle(no_default_fn(defaultness));

        errors.finish_with(SomeFunc { attrs, vis, sig })
    }

    fn trait_func(item: TraitItemFn) -> Result<SomeFunc> {
        let TraitItemFn {
            attrs,
            default,
            sig,
            ..
        } = item;

        let vis = Visibility::Inherited;

        let mut errors = Error::accumulator();

        errors.handle(no_fn_body(default));

        errors.finish_with(SomeFunc { attrs, vis, sig })
    }

    fn count_items(_: usize, _: usize) -> Result<()> {
        Ok(())
    }

    fn derive_func(SomeFunc { attrs, vis, sig }: SomeFunc, _: OuterType) -> Result<TokenStream> {
        let errors = Error::accumulator();

        let ((FlagLike(prop), attrs), errors) =
            FlagLike::<JsProperty>::exactly_one(attrs, sig.ident.span()).or_fatal(errors)?;

        let (impl_, errors) = match prop {
            JsProperty::Prop(FlagLike(prop)) => {
                prop::impl_property(prop, sig).error_at::<JsProperty, Property>()
            }
            JsProperty::Func(FlagLike(func)) => {
                func::impl_function(func.into(), sig).error_at::<JsProperty, Function>()
            }
            JsProperty::New(FlagLike(ctor)) => {
                func::impl_function(ctor.into(), sig).error_at::<JsProperty, Constructor>()
            }
            JsProperty::Get(FlagLike(getter)) => {
                get::impl_getter(getter, sig).error_at::<JsProperty, Getter>()
            }
            JsProperty::Set(FlagLike(setter)) => {
                set::impl_setter(setter, sig).error_at::<JsProperty, Setter>()
            }
        }
        .or_fatal(errors)?;

        errors.finish()?;

        let impl_ = impl_
            .into_iter()
            .map(|impl_| quote! { #(#attrs)* #vis #impl_ });

        Ok(quote! { #(#impl_)* })
    }

    fn unsupported<T, S: Spanned>(span: S) -> Result<T> {
        "only fn items are supported\nfn should have an empty body\nmove this item to another impl block"
        .pipe(Error::custom).with_span(&span)
        .pipe(Err)
    }

    fn impl_type(item: ImplItemType) -> Result<SomeType> {
        Self::unsupported(item)
    }

    fn trait_type(item: TraitItemType) -> Result<SomeType> {
        Self::unsupported(item)
    }

    fn derive_type(item: SomeType, _: OuterType) -> Result<TokenStream> {
        Self::unsupported(item.ident)
    }
}

fn property_key(src: &Ident, alt: Option<PropertyKey>) -> PropertyKey {
    match alt {
        Some(key) => key,
        None => src
            .to_string()
            .to_lower_camel_case()
            .pipe(PropertyKey::String),
    }
}

struct ResolveName<'a> {
    ident: &'a Ident,
    name: Option<PropKeyString>,
    symbol: Option<PropKeySymbol>,
}

impl ResolveName<'_> {
    fn resolve(self) -> Caveat<PropertyKey> {
        let Self {
            ident,
            name,
            symbol,
        } = self;

        let mut error = None;

        let specified = match (name, symbol) {
            (None, None) => None,
            (Some(StringLike(name)), None) => Some(PropertyKey::String(name)),
            (None, Some(StringLike(symbol))) => Some(PropertyKey::Symbol(symbol)),
            (Some(StringLike(name)), Some(_)) => {
                error = Error::custom("cannot specify both a name and a symbol")
                    .with_span(&ident)
                    .pipe(Some);
                Some(PropertyKey::String(name))
            }
        };

        let resolved = match specified {
            Some(name) => name,
            None => ident
                .to_string()
                .to_lower_camel_case()
                .pipe(PropertyKey::String),
        };

        (resolved, error).into()
    }
}

impl FlagName for JsProperty {
    const PREFIX: &'static str = "js";

    fn unit() -> Result<Self> {
        JsProperty::from_word()
    }
}

impl FlagEnum for JsProperty {
    const PREFIXES: &'static [&'static str] =
        &[Property::PREFIX, Function::PREFIX, Constructor::PREFIX];
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
    const PREFIX: &'static str = "get";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}

impl FlagName for Setter {
    const PREFIX: &'static str = "set";

    fn unit() -> Result<Self> {
        Ok(Self)
    }
}
