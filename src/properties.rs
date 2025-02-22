use darling::{error::Accumulator, util::Flag, FromMeta, Result};
use heck::ToLowerCamelCase;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, Parser},
    punctuated::Punctuated,
    FnArg, Generics, Ident, ImplItem, ImplItemFn, ItemImpl, Lit, Meta, Receiver, Signature, Token,
};
use tap::Pipe;

use crate::{
    util::{
        use_prelude, FatalErrors, Feature, FeatureEnum, FeatureName, FromPositional, MergeErrors,
        Positional, PropertyKey,
    },
    Properties,
};

mod function;
mod property;

pub fn properties(_: Properties, item: TokenStream) -> Result<TokenStream> {
    let errors = Accumulator::default();

    let (item, mut errors) = ItemImpl::parse.parse2(item).or_fatal(errors)?;

    errors.handle(only_inherent_impl(&item));

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

    errors.finish()?;

    Ok(quote! {
        const _: () = {
            #use_prelude

            #[allow(unused)]
            use deno_core::{
                anyhow::{anyhow, Context, Result}, error::JsError,
                ascii_str, serde_v8, v8, FastString, JsRuntime,
            };

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

    let ((Feature(prop), attrs), errors) =
        Feature::<JsProperty>::exactly_one(attrs, sig.ident.span()).or_fatal(errors)?;

    let (impl_, errors) = match prop {
        JsProperty::Prop(Feature(prop)) => property::impl_property(prop, sig),
        JsProperty::Func(Feature(func)) => function::impl_function(func.into(), sig),
        JsProperty::New(Feature(ctor)) => function::impl_function(ctor.into(), sig),
    }
    .or_fatal(errors)?;

    errors.finish()?;

    let impl_ = impl_
        .into_iter()
        .map(|impl_| quote! { #(#attrs)* #vis #impl_ });

    Ok(quote! { #(#impl_)* })
}

#[derive(Debug, Clone, FromMeta)]
enum JsProperty {
    Prop(Feature<Property>),
    Func(Feature<Function>),
    New(Feature<Constructor>),
}

#[derive(Debug, Default, Clone)]
struct PropertyName(Option<PropertyKey<String>>);

#[derive(Debug, Default, Clone, FromMeta)]
struct Property(Positional<PropertyName, PropertyOptions>);

#[derive(Debug, Default, Clone, FromMeta)]
struct PropertyOptions {
    with_setter: Flag,
}

#[derive(Debug, Default, Clone, FromMeta)]
struct Function(Positional<PropertyName, FunctionOptions>);

#[derive(Debug, Default, Clone, FromMeta)]
struct FunctionOptions {
    #[darling(default)]
    this: This,
}

#[derive(Debug, Default, Clone, FromMeta)]
struct Constructor {
    class: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, FromMeta)]
enum This {
    #[darling(rename = "self")]
    #[default]
    Self_,
    #[darling(rename = "undefined")]
    Undefined,
}

impl FromMeta for PropertyName {
    fn from_meta(item: &Meta) -> Result<Self> {
        Ok(Self(Some(<_>::from_meta(item)?)))
    }

    fn from_value(value: &Lit) -> Result<Self> {
        Ok(Self(Some(<_>::from_value(value)?)))
    }

    fn from_none() -> Option<Self> {
        Some(Self(None))
    }
}

impl FromPositional for PropertyName {
    fn fallback() -> Result<Self> {
        Ok(Self(None))
    }
}

#[derive(Debug, Clone, Copy)]
enum MaybeAsync {
    Sync,
    Async(Token![async]),
}

impl MaybeAsync {
    fn only<F: FeatureName>(self, sig: &Signature) -> (Self, Option<darling::Error>) {
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

    fn some<F: FeatureName>(sig: &Signature) -> (Self, Option<darling::Error>) {
        let color = match &sig.asyncness {
            None => MaybeAsync::Sync,
            Some(token) => MaybeAsync::Async(*token),
        };
        (color, Self::supported::<F>(sig).into_one())
    }

    fn supported<F: FeatureName>(sig: &Signature) -> Accumulator {
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

fn only_inherent_impl(item: &ItemImpl) -> Result<()> {
    let mut errors = Accumulator::default();

    if item.defaultness.is_some() {
        Properties::error("impl cannot be `default`")
            .with_span(&item.defaultness)
            .pipe(|e| errors.push(e));
    }

    if item.unsafety.is_some() {
        Properties::error("impl cannot be `unsafe`")
            .with_span(&item.unsafety)
            .pipe(|e| errors.push(e));
    }

    if let Some((_, ty, _)) = &item.trait_ {
        Properties::error("cannot be a trait impl")
            .with_span(ty)
            .pipe(|e| errors.push(e));
    }

    errors.finish()
}

fn self_arg<F: FeatureName>(inputs: &Punctuated<FnArg, Token![,]>, sig: Span) -> Result<&Receiver> {
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

fn property_key(src: &Ident, alt: PropertyName) -> PropertyKey<String> {
    match alt.0 {
        Some(key) => key,
        None => src
            .to_string()
            .to_lower_camel_case()
            .pipe(PropertyKey::String),
    }
}

impl FeatureName for JsProperty {
    const PREFIX: &str = "js";

    fn unit() -> Result<Self> {
        JsProperty::from_word()
    }
}

impl FeatureEnum for JsProperty {
    const PREFIXES: &[&str] = &[Property::PREFIX, Function::PREFIX, Constructor::PREFIX];
}

impl FeatureName for Property {
    const PREFIX: &str = "prop";

    fn unit() -> Result<Self> {
        Ok(Default::default())
    }
}

impl FeatureName for Function {
    const PREFIX: &str = "func";

    fn unit() -> Result<Self> {
        Ok(Default::default())
    }
}

impl FeatureName for Constructor {
    const PREFIX: &str = "new";

    fn unit() -> Result<Self> {
        Ok(Default::default())
    }
}
