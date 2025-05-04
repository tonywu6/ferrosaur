use darling::{Error, Result};
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
        flag::{FlagError, FlagLike},
        interface::{DeriveInterface, InterfaceLike, OuterType, SomeFunc, SomeType},
        no_default_fn, no_fn_body,
        property::PropertyKey,
        string::StringLike,
        Caveat, FatalErrors,
    },
    Constructor, Function, Getter, Interface, JsProp, PropKeyString, PropKeySymbol, Property,
    Setter,
};

mod func;
mod index;
mod prop;

pub fn interface(_: Interface, item: TokenStream) -> Result<TokenStream> {
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
            FlagLike::<JsProp>::exactly_one(attrs, sig.ident.span()).or_fatal(errors)?;

        let (impl_, errors) = match prop {
            JsProp::Prop(FlagLike(prop)) => {
                prop::impl_property(prop, sig).error_at::<JsProp, Property>()
            }
            JsProp::Func(FlagLike(func)) => {
                func::impl_function(func.into(), sig).error_at::<JsProp, Function>()
            }
            JsProp::New(FlagLike(ctor)) => {
                func::impl_function(ctor.into(), sig).error_at::<JsProp, Constructor>()
            }
            JsProp::GetIndex(FlagLike(getter)) => {
                index::impl_getter(getter, sig).error_at::<JsProp, Getter>()
            }
            JsProp::SetIndex(FlagLike(setter)) => {
                index::impl_setter(setter, sig).error_at::<JsProp, Setter>()
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
