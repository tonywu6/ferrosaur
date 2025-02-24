use darling::{error::Accumulator, util::Flag, Error, FromMeta, Result};
use heck::ToLowerCamelCase;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, Parser},
    punctuated::Punctuated,
    FnArg, Generics, Ident, ImplItem, ImplItemFn, ItemImpl, Receiver, Signature, Token,
};
use tap::Pipe;

use crate::{
    util::{
        only_inherent_impl, use_deno, use_prelude, FatalErrors, FlagEnum, FlagLike, FlagName,
        FunctionThis, MergeErrors, PropertyKey, StringLike, Unary, WellKnown,
    },
    Properties,
};

mod func;
mod prop;

#[derive(Debug, Clone, FromMeta)]
enum JsProperty {
    Prop(FlagLike<Property>),
    Func(FlagLike<Function>),
    New(FlagLike<Constructor>),
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
struct Constructor {
    class: Option<Unary<PropKeyString>>,
}

pub fn properties(_: Properties, item: TokenStream) -> Result<TokenStream> {
    let errors = Accumulator::default();

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
    let ImplItem::Fn(func) = item else {
        return "only fn items are supported\nmove this item to another impl block"
            .pipe(Properties::error)
            .with_span(&item)
            .pipe(Err);
    };

    let mut errors = Accumulator::default();

    let ImplItemFn {
        attrs,
        vis,
        defaultness,
        sig,
        block,
    } = func;

    errors.handle(if !block.stmts.is_empty() {
        Properties::error("macro ignores fn body\nchange this to {}")
            .with_span(&block)
            .pipe(Err)
    } else {
        Ok(())
    });

    errors.handle(if defaultness.is_some() {
        Properties::error("fn cannot be `default` here")
            .with_span(&defaultness)
            .pipe(Err)
    } else {
        Ok(())
    });

    let ((FlagLike(prop), attrs), errors) =
        FlagLike::<JsProperty>::exactly_one(attrs, sig.ident.span()).or_fatal(errors)?;

    let (impl_, errors) = match prop {
        JsProperty::Prop(FlagLike(prop)) => prop::impl_property(prop, sig),
        JsProperty::Func(FlagLike(func)) => func::impl_function(func.into(), sig),
        JsProperty::New(FlagLike(ctor)) => func::impl_function(ctor.into(), sig),
    }
    .or_fatal(errors)?;

    errors.finish()?;

    let impl_ = impl_
        .into_iter()
        .map(|impl_| quote! { #(#attrs)* #vis #impl_ });

    Ok(quote! { #(#impl_)* })
}

#[derive(Debug, Clone, Copy)]
enum MaybeAsync {
    Sync,
    Async(Token![async]),
}

impl MaybeAsync {
    fn only<F: FlagName>(self, sig: &Signature) -> (Self, Option<darling::Error>) {
        let mut errors = Accumulator::default();

        let (color, error) = Self::some::<F>(sig);

        if let Some(error) = error {
            errors.push(error);
        }

        match self {
            MaybeAsync::Sync => {
                if let MaybeAsync::Async(span) = color {
                    F::error("fn cannot be `async` here")
                        .with_span(&span)
                        .pipe(|e| errors.push(e));
                }
            }
            MaybeAsync::Async(_) => {
                if matches!(color, MaybeAsync::Sync) {
                    F::error("fn is required to be `async` here")
                        .with_span(&sig.fn_token)
                        .pipe(|e| errors.push(e));
                }
            }
        }

        (color, errors.into_one())
    }

    fn some<F: FlagName>(sig: &Signature) -> (Self, Option<darling::Error>) {
        let color = match &sig.asyncness {
            None => MaybeAsync::Sync,
            Some(token) => MaybeAsync::Async(*token),
        };
        (color, Self::supported::<F>(sig).into_one())
    }

    fn supported<F: FlagName>(sig: &Signature) -> Accumulator {
        let mut errors = Accumulator::default();

        macro_rules! deny {
            ($attr:ident, $msg:literal) => {
                if sig.$attr.is_some() {
                    F::error($msg)
                        .with_span(&sig.$attr)
                        .pipe(|e| errors.push(e));
                }
            };
        }

        deny!(constness, "fn cannot be `const` here");
        deny!(unsafety, "fn cannot be `unsafe` here");
        deny!(abi, "fn cannot be `extern` here");
        deny!(variadic, "fn cannot be variadic here");

        errors
    }
}

impl ToTokens for MaybeAsync {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Async(token) => tokens.extend(quote! { #token }),
            Self::Sync => {}
        }
    }
}

fn self_arg<F: FlagName>(inputs: &Punctuated<FnArg, Token![,]>, sig: Span) -> Result<&Receiver> {
    match inputs.first() {
        Some(FnArg::Receiver(recv)) => {
            if recv.reference.is_none() || recv.mutability.is_some() {
                F::error("must be `&self`").with_span(recv).pipe(Err)
            } else {
                Ok(recv)
            }
        }
        Some(FnArg::Typed(ty)) => F::error("must have `&self` as the first argument")
            .with_span(ty)
            .pipe(Err),
        None => F::error("missing `&self` as the first argument")
            .with_span(&sig)
            .pipe(Err),
    }
}

fn name_or_symbol<F: FlagName>(
    span: Span,
    name: Option<PropKeyString>,
    symbol: Option<PropKeySymbol>,
) -> (Option<PropertyKey<String>>, Option<Error>) {
    match (name, symbol) {
        (None, None) => (None, None),
        (Some(StringLike(name)), None) => (Some(PropertyKey::String(name)), None),
        (None, Some(StringLike(symbol))) => (Some(PropertyKey::Symbol(symbol)), None),
        (Some(StringLike(name)), Some(_)) => (
            Some(PropertyKey::String(name)),
            Some(F::error("cannot specify both a name and a symbol").with_span(&span)),
        ),
    }
}

fn property_key(src: &Ident, alt: Option<PropertyKey<String>>) -> PropertyKey<String> {
    match alt {
        Some(key) => key,
        None => src
            .to_string()
            .to_lower_camel_case()
            .pipe(PropertyKey::String),
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
