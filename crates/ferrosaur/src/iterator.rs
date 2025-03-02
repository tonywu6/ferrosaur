use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Generics, Ident, ImplItemFn, ImplItemType, Token, TraitItemFn, TraitItemType,
    TypeParamBound, Visibility,
};
use tap::Pipe;

use crate::{
    util::{
        function::{BindFunction, FunctionLength, FunctionThis},
        interface::{DeriveInterface, InterfaceLike, OuterType, OuterTypeKind, SomeFunc, SomeType},
        property::PropertyKey,
        type_ident,
        v8::{to_v8_bound, V8Conv},
        FatalErrors, MergeGenerics, RecoverableErrors,
    },
    Iterator_,
};

pub fn iterator(_: Iterator_, item: TokenStream) -> Result<TokenStream> {
    InterfaceLike::parse
        .parse2(item)?
        .derive::<DeriveIterator>()
}

struct DeriveIterator;

impl DeriveInterface for DeriveIterator {
    fn impl_type(item: ImplItemType) -> Result<SomeType> {
        let ImplItemType {
            attrs,
            defaultness,
            vis,
            ident,
            generics,
            ty,
            ..
        } = item;

        type_named_item(&ident)?;

        let mut errors = Error::accumulator();

        errors.handle(no_attributes(attrs));
        errors.handle(no_defaultness(defaultness));
        errors.handle(no_generics(&generics));
        errors.handle(no_where_clause(&generics));
        errors.handle(no_visibility(vis));

        errors.finish_with(SomeType { ident, ty })
    }

    fn trait_type(item: TraitItemType) -> Result<SomeType> {
        let TraitItemType {
            attrs,
            ident,
            generics,
            bounds,
            default,
            ..
        } = item;

        type_named_item(&ident)?;

        let mut errors = Error::accumulator();

        errors.handle(no_attributes(attrs));
        errors.handle(no_generics(&generics));
        errors.handle(no_where_clause(&generics));
        errors.handle(no_bounds(bounds));

        let (ty, errors) = match default {
            None => Error::custom("a concrete type is required: `type Item = ...`")
                .with_span(&ident)
                .pipe(Err),
            Some((_, ty)) => Ok(ty),
        }
        .or_fatal(errors)?;

        errors.finish_with(SomeType { ident, ty })
    }

    fn count_items(fns: usize, types: usize) -> Result<()> {
        match (fns, types) {
            (_, 1) => Ok(()),
            (_, _) => "expected exactly one `type Item = ...`"
                .pipe(Error::custom)
                .pipe(Err),
        }
    }

    fn derive_type(
        SomeType { ty, .. }: SomeType,
        OuterType {
            this,
            generics,
            kind,
        }: OuterType,
    ) -> Result<TokenStream> {
        let mut errors = Error::accumulator();

        let item_type = V8Conv::from_type(ty).and_recover(&mut errors);

        let return_ty = item_type.to_type();

        let fn_value = BindFunction {
            source: "next".into(),
            this: FunctionThis::Self_,
            ctor: false,
            length: FunctionLength::Fixed(0),
        };

        let value_key = PropertyKey::from("value");
        let value_getter = V8Conv::default().to_getter(&Default::default());

        let done_key = PropertyKey::from("done");
        let done_getter = V8Conv::default().to_getter(&Default::default());

        let into_item = item_type.to_cast_from_v8("value", "scope");

        let vis = match kind {
            OuterTypeKind::Impl => quote! { pub },
            OuterTypeKind::Trait => quote! {},
        };

        let fn_next = quote! {
            #vis fn next(&mut self, rt: &mut JsRuntime) -> Result<Option<#return_ty>>
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
            let iter_lifetime = quote! { '_iter };
            let trait_generic = format_ident!("_Inner");

            let capturing = match kind {
                OuterTypeKind::Impl => quote! { + use<#iter_lifetime> },
                OuterTypeKind::Trait => quote! {},
            };

            let inner_type = match kind {
                OuterTypeKind::Impl => quote! { #this },
                OuterTypeKind::Trait => quote! { #trait_generic },
            };

            let generics = match kind {
                OuterTypeKind::Impl => MergeGenerics {
                    outer: generics,
                    lifetimes: vec![quote! { #iter_lifetime }],
                    types: vec![],
                    bounds: vec![],
                },
                OuterTypeKind::Trait => {
                    let outer_args = MergeGenerics {
                        outer: generics,
                        lifetimes: vec![],
                        types: vec![],
                        bounds: vec![],
                    }
                    .arguments();
                    MergeGenerics {
                        outer: generics,
                        lifetimes: vec![quote! { #iter_lifetime }],
                        types: vec![quote! { #trait_generic }],
                        bounds: vec![
                            to_v8_bound(type_ident(trait_generic.clone())).to_token_stream(),
                            quote! { #trait_generic: #this <#outer_args> },
                        ],
                    }
                }
            };

            let params = generics.params();
            let bounds = generics.bounds();
            let arguments = generics.arguments();

            let phantom_fields = generics.phantom_fields();
            let phantom_init = generics.phantom_init();

            quote! {
                #vis fn into_iter<#iter_lifetime>(
                    self,
                    rt: &#iter_lifetime mut JsRuntime,
                ) -> impl Iterator<Item = Result<#return_ty>> #capturing
                {
                    struct Iter <#params> {
                        rt: &#iter_lifetime mut JsRuntime,
                        inner: #inner_type,
                        #phantom_fields
                    }

                    impl <#params> ::core::iter::Iterator for Iter <#arguments>
                    where
                        #bounds
                    {
                        type Item = Result<#return_ty>;

                        fn next(&mut self) -> Option<Self::Item> {
                            self.inner.next(self.rt).transpose()
                        }
                    }

                    Iter { rt, inner: self, #phantom_init }
                }
            }
        };

        errors.finish_with(quote! {
            #fn_next
            #fn_into_iter
        })
    }

    fn unsupported<T, S: Spanned>(span: S) -> Result<T> {
        Error::custom("unexpected item\nmove this item to another impl block")
            .with_span(&span)
            .pipe(Err)
    }

    fn impl_func(item: ImplItemFn) -> Result<SomeFunc> {
        Self::unsupported(item)
    }

    fn trait_func(item: TraitItemFn) -> Result<SomeFunc> {
        Self::unsupported(item)
    }

    fn derive_func(item: SomeFunc, _: OuterType) -> Result<TokenStream> {
        Self::unsupported(item.sig.ident)
    }
}

fn type_named_item(ident: &Ident) -> Result<()> {
    if ident != "Item" {
        "unexpected type name, expected `type Item`"
            .pipe(Error::custom)
            .with_span(&ident)
            .pipe(Err)
    } else {
        Ok(())
    }
}

fn no_attributes(attrs: Vec<Attribute>) -> Result<()> {
    if !attrs.is_empty() {
        Error::custom("macro ignores attributes in this location")
            .with_span(&quote! { #(#attrs)* })
            .pipe(Err)
    } else {
        Ok(())
    }
}

fn no_defaultness(defaultness: Option<Token![default]>) -> Result<()> {
    if defaultness.is_some() {
        Error::custom("macro ignores `default` in this location")
            .with_span(&defaultness)
            .pipe(Err)
    } else {
        Ok(())
    }
}

fn no_generics(generics: &Generics) -> Result<()> {
    if !generics.params.is_empty() {
        Error::custom("macro ignores type params in this location")
            .with_span(&generics.params)
            .pipe(Err)
    } else {
        Ok(())
    }
}

fn no_where_clause(generics: &Generics) -> Result<()> {
    if generics.where_clause.is_some() {
        Error::custom("macro ignores where clause in this location")
            .with_span(&generics.where_clause)
            .pipe(Err)
    } else {
        Ok(())
    }
}

fn no_visibility(vis: Visibility) -> Result<()> {
    if matches!(vis, Visibility::Public(_) | Visibility::Restricted(_)) {
        Error::custom("macro ignores visibility qualifiers in this location")
            .with_span(&vis)
            .pipe(Err)
    } else {
        Ok(())
    }
}

fn no_bounds(bounds: Punctuated<TypeParamBound, Token![+]>) -> Result<()> {
    if bounds.is_empty() {
        Ok(())
    } else {
        Error::custom("macro ignores trait bounds in this location")
            .with_span(&bounds)
            .pipe(Err)
    }
}
