use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, Parser},
    Generics, Ident, ImplItem, ImplItemType, ItemImpl, Visibility,
};
use tap::Pipe;

use crate::{
    util::{
        only_inherent_impl, use_deno, use_prelude, BindFunction, FatalErrors, FunctionLength,
        FunctionThis, PropertyKey, RecoverableErrors, V8Conv,
    },
    Iterator_,
};

pub fn iterator(_: Iterator_, item: TokenStream) -> Result<TokenStream> {
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

    let item_type = items
        .into_iter()
        .filter_map(|item| errors.handle(item_type(item)))
        .collect::<Vec<_>>();

    let item_type = match item_type.len() {
        0 => V8Conv::default(), // TODO: make this mandatory
        1 => item_type.into_iter().next().unwrap().0,
        _ => {
            let mut item_type = item_type.into_iter();
            let ty = item_type.next().unwrap().0;
            "more than 1 `type Item` specified\nspecify 1 or none"
                .pipe(Error::custom)
                .with_span(&item_type.next().unwrap().1)
                .pipe(|e| errors.push(e));
            ty
        }
    };

    let return_ty = item_type.to_type();

    let fn_value = BindFunction {
        source: "next".into(),
        this: FunctionThis::Self_,
        ctor: false,
        length: FunctionLength::Fixed(0),
    };

    let value_key = PropertyKey::from("value");
    let value_getter = V8Conv::default().to_getter();

    let done_key = PropertyKey::from("done");
    let done_getter = V8Conv::default().to_getter();

    let into_item = item_type.to_cast_from_v8("value", "scope");

    let fn_next = quote! {
        pub fn next(&mut self, rt: &mut JsRuntime) -> Result<Option<#return_ty>>
        {
            let scope = &mut rt.handle_scope();
            let next = {
                #fn_value
                let this = ToV8::to_v8(&*self, scope)?;
                let this = v8::Local::new(scope, this);
                call(scope, this, [])
                    .context("failed to call `next` on iterator")?
            };
            let done = {
                #done_getter
                let this = v8::Local::new(scope, &next);
                let prop = #done_key;
                getter(scope, this, prop)
                    .context("failed to get `done` from iterator result")?
            };
            let value = {
                #value_getter
                let this = v8::Local::new(scope, &next);
                let prop = #value_key;
                getter(scope, this, prop)
                    .context("failed to get `value` from iterator result")?
            };
            let done = v8::Local::new(scope, done);
            let value = v8::Local::new(scope, value);
            if done.is_true() {
                if value.is_undefined() {
                    Ok(None)
                } else {
                    Ok(Some(#into_item?))
                }
            } else {
                Ok(Some(#into_item?))
            }
        }
    };

    let fn_into_iter = {
        quote! {
            pub fn into_iter(self, rt: &mut JsRuntime)
                -> impl Iterator<Item = Result<#return_ty>> + use<'_, #params>
            {
                struct Iter<'__rt, #params> {
                    rt: &'__rt mut JsRuntime,
                    inner: #self_ty,
                }

                impl < #params> ::core::iter::Iterator for Iter<'_, #params>
                #where_clause
                {
                    type Item = Result<#return_ty>;

                    fn next(&mut self) -> Option<Self::Item> {
                        self.inner.next(self.rt).transpose()
                    }
                }

                Iter { rt, inner: self }
            }
        }
    };

    errors.finish()?;

    Ok(quote! {
        const _: () = {
            #use_prelude
            #use_deno

            #(#attrs)*
            impl <#params> #self_ty
            #where_clause
            {
                #fn_next
                #fn_into_iter
            }
        };
    })
}

fn item_type(item: ImplItem) -> Result<(V8Conv, Ident)> {
    let ImplItem::Type(ty) = item else {
        return "unexpected item\nmove this item to another impl block"
            .pipe(Error::custom)
            .with_span(&item)
            .pipe(Err);
    };

    let ImplItemType {
        attrs,
        defaultness,
        vis,
        ident,
        generics,
        ty,
        ..
    } = ty;

    if ident != "Item" {
        return "unexpected type name, expected `type Item`"
            .pipe(Error::custom)
            .with_span(&ident)
            .pipe(Err);
    }

    let mut errors = Error::accumulator();

    errors.handle(if !attrs.is_empty() {
        Error::custom("macro ignores attributes in this location")
            .with_span(&quote! { #(#attrs)* })
            .pipe(Err)
    } else {
        Ok(())
    });

    errors.handle(if defaultness.is_some() {
        Error::custom("macro ignores `default` in this location")
            .with_span(&defaultness)
            .pipe(Err)
    } else {
        Ok(())
    });

    errors.handle(if !generics.params.is_empty() {
        Error::custom("macro ignores type params in this location")
            .with_span(&generics.params)
            .pipe(Err)
    } else {
        Ok(())
    });

    errors.handle(if generics.where_clause.is_some() {
        Error::custom("macro ignores where clause in this location")
            .with_span(&generics.where_clause)
            .pipe(Err)
    } else {
        Ok(())
    });

    errors.handle(
        if matches!(vis, Visibility::Public(_) | Visibility::Restricted(_)) {
            Error::custom("macro ignores visibility modifiers in this location")
                .with_span(&vis)
                .pipe(Err)
        } else {
            Ok(())
        },
    );

    let ty = V8Conv::from_type(ty).and_recover(&mut errors);

    errors.finish_with((ty, ident))
}
