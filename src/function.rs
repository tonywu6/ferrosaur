use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, Parser},
    Generics, ImplItem, ImplItemFn, ItemImpl,
};
use tap::Pipe;

use crate::{
    util::{
        expect_self_arg, only_impl_fn, only_inherent_impl, use_deno, use_prelude, CallFunction,
        FatalErrors, FlagName, FunctionSource, FunctionThis, RecoverableErrors,
    },
    Function, JsItem,
};

pub fn function(_: Function, item: TokenStream) -> Result<TokenStream> {
    let errors = Error::accumulator();

    let (item, mut errors) = ItemImpl::parse.parse2(item).or_fatal(errors)?;

    errors.handle(only_inherent_impl(&item));

    let ItemImpl {
        attrs,
        generics: Generics {
            params,
            where_clause,
            ..
        },
        self_ty,
        items,
        ..
    } = item;

    let items = items
        .into_iter()
        .filter_map(|item| errors.handle(filter_call(item)))
        .collect::<Vec<_>>();

    let (item, errors) = match items.len() {
        1 => items.into_iter().next().unwrap().pipe(Ok),
        _ => Error::custom("expected exactly one fn named `call`")
            .with_span(&self_ty)
            .pipe(Err),
    }
    .or_fatal(errors)?;

    let (item, errors) = impl_call(item).or_fatal(errors)?;

    errors.finish()?;

    Ok(quote! {
        const _: () = {
            #use_prelude
            #use_deno

            #[automatically_derived]
            #(#attrs)*
            impl <#params> #self_ty
            #where_clause
            {
                #item
            }
        };
    })
}

fn filter_call(item: ImplItem) -> Result<ImplItemFn> {
    let func = only_impl_fn(item)?;

    let mut errors = Error::accumulator();

    errors.handle(if !func.block.stmts.is_empty() {
        Error::custom("macro ignores fn body\nchange this to {}")
            .with_span(&func.block)
            .pipe(Err)
    } else {
        Ok(())
    });

    errors.handle(if func.defaultness.is_some() {
        Error::custom("fn cannot be `default` here")
            .with_span(&func.defaultness)
            .pipe(Err)
    } else {
        Ok(())
    });

    errors.handle(if func.sig.ident != "call" {
        Error::custom("expected fn to be named `call`")
            .with_span(&func.sig.ident)
            .pipe(Err)
    } else {
        Ok(())
    });

    func.attrs.iter().for_each(|attr| {
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

    errors.finish_with(func)
}

fn impl_call(
    ImplItemFn {
        attrs,
        vis,
        mut sig,
        ..
    }: ImplItemFn,
) -> Result<TokenStream> {
    let mut errors = Error::accumulator();

    let mut call = CallFunction::from_sig(&mut sig).and_recover(&mut errors);

    call.source = FunctionSource::This;
    call.this = FunctionThis::Undefined;

    let fn_self = errors.handle(expect_self_arg(&sig.inputs, &sig.ident));

    let impl_ = call.render(fn_self, &sig.ident, &sig.generics);

    errors.finish_with(quote! { #(#attrs)* #vis #impl_ })
}
