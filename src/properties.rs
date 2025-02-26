use darling::{util::Flag, Error, FromMeta, Result};
use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, Parser},
    punctuated::Punctuated,
    FnArg, Generics, Ident, ImplItem, ImplItemFn, ItemImpl, Receiver, Token,
};
use tap::Pipe;

use crate::{
    util::{
        only_fn_item, only_inherent_impl, use_deno, use_prelude, Caveat, ErrorLocation,
        FatalErrors, FlagEnum, FlagLike, FlagName, FunctionThis, PropertyKey, StringLike, Unary,
        WellKnown,
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
    #[darling(default)]
    this: FunctionThis,
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
    let errors = Error::accumulator();

    let (item, mut errors) = ItemImpl::parse.parse2(item).or_fatal(errors)?;

    errors.handle(only_inherent_impl::<Properties>(&item));

    let ItemImpl {
        attrs,
        generics,
        self_ty,
        items,
        ..
    } = item;

    let Generics {
        params,
        where_clause,
        ..
    } = generics;

    let items = items
        .into_iter()
        .filter_map(|item| errors.handle(impl_item(item)))
        .collect::<Vec<_>>();

    let use_prelude = use_prelude();

    let use_deno = use_deno();

    errors.finish()?;

    Ok(quote! {
        const _: () = {
            #use_prelude
            #use_deno

            #(#attrs)*
            impl <#params> #self_ty
            #where_clause
            {
                #(#items)*
            }
        };
    })
}

fn impl_item(item: ImplItem) -> Result<TokenStream> {
    let func = only_fn_item(item)?;

    let mut errors = Error::accumulator();

    let ImplItemFn {
        attrs,
        vis,
        defaultness,
        sig,
        block,
    } = func;

    errors.handle(if !block.stmts.is_empty() {
        Error::custom("macro ignores fn body\nchange this to {}")
            .with_span(&block)
            .pipe(Err)
    } else {
        Ok(())
    });

    errors.handle(if defaultness.is_some() {
        Error::custom("fn cannot be `default` here")
            .with_span(&defaultness)
            .pipe(Err)
    } else {
        Ok(())
    });

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

fn self_arg<'a>(inputs: &'a Punctuated<FnArg, Token![,]>, ident: &Ident) -> Result<&'a Receiver> {
    match inputs.first() {
        Some(FnArg::Receiver(recv)) => {
            if recv.reference.is_none() || recv.mutability.is_some() {
                Error::custom("must be `&self`").with_span(recv).pipe(Err)
            } else {
                Ok(recv)
            }
        }
        Some(FnArg::Typed(ty)) => Error::custom("must have `&self` as the first argument")
            .with_span(ty)
            .pipe(Err),
        None => Error::custom("missing `&self` as the first argument")
            .with_span(ident)
            .pipe(Err),
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
