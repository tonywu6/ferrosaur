use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, Parser},
    spanned::Spanned,
    Attribute, ImplItemFn, ImplItemType, Signature, TraitItemFn, TraitItemType, Visibility,
};
use tap::{Pipe, Tap};

use crate::{
    util::{
        expect_self_arg, no_default_fn, no_fn_body, CallFunction, DeriveInterface, FlagName,
        FunctionSource, InterfaceLike, OuterType, RecoverableErrors, SomeFunc, SomeType,
    },
    Function, JsItem,
};

pub fn function(_: Function, item: TokenStream) -> Result<TokenStream> {
    InterfaceLike::parse
        .parse2(item)?
        .derive::<DeriveFunction>()
}

struct DeriveFunction;

impl DeriveInterface for DeriveFunction {
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
        errors.handle(expected_fn_call(&sig));
        errors.handle(expected_no_attr(&attrs));

        errors.finish_with(SomeFunc { attrs, vis, sig })
    }

    fn trait_func(item: TraitItemFn) -> Result<SomeFunc> {
        let TraitItemFn {
            attrs,
            default,
            sig,
            ..
        } = item;

        let mut errors = Error::accumulator();

        errors.handle(no_fn_body(default));
        errors.handle(expected_fn_call(&sig));
        errors.handle(expected_no_attr(&attrs));

        let vis = Visibility::Inherited;

        errors.finish_with(SomeFunc { attrs, vis, sig })
    }

    fn count_items(fns: usize, types: usize) -> Result<()> {
        match (fns, types) {
            (1, _) => Ok(()),
            (_, _) => Err(Error::custom("expected exactly one fn named `call`")),
        }
    }

    fn derive_func(
        SomeFunc {
            attrs,
            vis,
            mut sig,
        }: SomeFunc,
        _: OuterType,
    ) -> Result<TokenStream> {
        let mut errors = Error::accumulator();

        let call = CallFunction::from_sig(&mut sig)
            .and_recover(&mut errors)
            .tap_mut(|call| call.source = FunctionSource::This);

        let fn_self = errors.handle(expect_self_arg(&sig.inputs, &sig.ident));

        let rendered = call.render(fn_self, &sig.ident, &sig.generics);

        errors.finish_with(quote! { #(#attrs)* #vis #rendered })
    }

    fn unsupported<T, S: Spanned>(item: S) -> Result<T> {
        Error::custom("expected exactly one fn named `call`")
            .with_span(&item)
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

fn expected_fn_call(sig: &Signature) -> Result<()> {
    if sig.ident != "call" {
        Error::custom("expected fn to be named `call`")
            .with_span(&sig.ident)
            .pipe(Err)
    } else {
        Ok(())
    }
}

fn expected_no_attr(attrs: &[Attribute]) -> Result<()> {
    let mut errors = Error::accumulator();

    attrs.iter().for_each(|attr| {
        let js_attr = attr
            .path()
            .segments
            .get(0)
            .map(|s| s.ident == JsItem::PREFIX);
        if matches!(js_attr, Some(true)) {
            Error::custom("a #[js(...)] is not required here")
                .with_span(&attr)
                .pipe(|e| errors.push(e));
        }
    });

    errors.finish()
}
