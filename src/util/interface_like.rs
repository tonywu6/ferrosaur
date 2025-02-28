use darling::{Error, Result};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Generics, Ident, ImplItem, ImplItemFn, ImplItemType, ItemImpl, ItemTrait, Signature,
    Token, TraitItem, TraitItemFn, TraitItemType, Type, Visibility,
};
use tap::{Pipe, Tap};

use super::{
    empty_where_clause, inner_mod_name, only_inherent_impl, only_regular_trait, to_v8_bound,
    type_ident, use_deno, use_prelude,
};

#[derive(Debug)]
pub enum InterfaceLike {
    Impl(ItemImpl),
    Trait(ItemTrait),
}

pub trait DeriveInterface {
    fn impl_func(item: ImplItemFn) -> Result<SomeFunc>;
    fn trait_func(item: TraitItemFn) -> Result<SomeFunc>;
    fn impl_type(item: ImplItemType) -> Result<SomeType>;
    fn trait_type(item: TraitItemType) -> Result<SomeType>;

    fn derive_func(item: SomeFunc, this: OuterType) -> Result<TokenStream>;
    fn derive_type(item: SomeType, this: OuterType) -> Result<TokenStream>;

    fn count_items(fns: usize, types: usize) -> Result<()>;
    fn unsupported<T, S: Spanned>(span: S) -> Result<T>;
}

pub struct SomeFunc {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: Signature,
}

pub struct SomeType {
    pub ident: Ident,
    pub ty: Type,
}

impl InterfaceLike {
    pub fn derive<D: DeriveInterface>(self) -> Result<TokenStream> {
        match self {
            Self::Impl(item) => Self::derive_impl::<D>(item),
            Self::Trait(item) => Self::derive_trait::<D>(item),
        }
    }

    fn derive_impl<D: DeriveInterface>(item: ItemImpl) -> Result<TokenStream> {
        let mut errors = Error::accumulator();

        errors.handle(only_inherent_impl(&item));

        let ItemImpl {
            attrs,
            generics,
            self_ty,
            items,
            ..
        } = item;

        let mut funcs = vec![];
        let mut types = vec![];

        let this = OuterType {
            kind: OuterTypeKind::Impl,
            this: &self_ty.to_token_stream(),
            generics: &generics,
        };

        for item in items {
            match item {
                ImplItem::Fn(item) => errors
                    .handle(D::impl_func(item))
                    .and_then(|item| errors.handle(D::derive_func(item, this)))
                    .map(|item| funcs.push(item)),
                ImplItem::Type(item) => errors
                    .handle(D::impl_type(item))
                    .and_then(|item| errors.handle(D::derive_type(item, this)))
                    .map(|item| types.push(item)),
                _ => errors.handle(D::unsupported(item)),
            };
        }

        D::count_items(funcs.len(), types.len())
            .map_err(|err| err.with_span(&self_ty))
            .pipe(|err| errors.handle(err));

        let Generics {
            params,
            where_clause,
            ..
        } = generics;

        errors.finish_with(quote! {
            const _: () = {
                #use_prelude
                #use_deno

                #[automatically_derived]
                #(#attrs)*
                impl <#params> #self_ty
                #where_clause
                {
                    #(#types)*
                    #(#funcs)*
                }
            };
        })
    }

    fn derive_trait<D: DeriveInterface>(item: ItemTrait) -> Result<TokenStream> {
        let mut errors = Error::accumulator();

        errors.handle(only_regular_trait(&item));

        let ItemTrait {
            attrs,
            vis,
            ident,
            generics,
            colon_token,
            supertraits,
            items,
            ..
        } = item;

        let mut funcs = vec![];
        let mut types = vec![];

        let this = OuterType {
            kind: OuterTypeKind::Trait,
            this: &ident.to_token_stream(),
            generics: &generics,
        };

        for item in items {
            match item {
                TraitItem::Fn(item) => errors
                    .handle(D::trait_func(item))
                    .and_then(|item| errors.handle(D::derive_func(item, this)))
                    .map(|item| funcs.push(item)),
                TraitItem::Type(item) => errors
                    .handle(D::trait_type(item))
                    .and_then(|item| errors.handle(D::derive_type(item, this)))
                    .map(|item| types.push(item)),
                _ => errors.handle(D::unsupported(item)),
            };
        }

        D::count_items(funcs.len(), types.len())
            .map_err(|err| err.with_span(&ident))
            .pipe(|err| errors.handle(err));

        let Generics {
            params,
            where_clause,
            ..
        } = generics;

        let inner_mod = inner_mod_name("trait", &ident);

        let self_ty = type_ident(Token![Self](Span::call_site()).into());

        let where_clause = where_clause
            .unwrap_or_else(empty_where_clause)
            .tap_mut(|clause| clause.predicates.push(to_v8_bound(self_ty)));

        errors.finish_with(quote! {
            #[doc(inline)]
            #vis use #inner_mod::#ident;

            #[doc(hidden)]
            mod #inner_mod {
                #[allow(unused)]
                use super::*;
                #use_prelude
                #use_deno

                #(#attrs)*
                pub trait #ident <#params> #colon_token #supertraits
                #where_clause
                {
                    #(#funcs)*
                    #(#types)*
                }
            }
        })
    }
}

#[derive(Clone, Copy)]
pub struct OuterType<'a> {
    pub kind: OuterTypeKind,
    pub this: &'a TokenStream,
    pub generics: &'a Generics,
}

#[derive(Clone, Copy)]
pub enum OuterTypeKind {
    Impl,
    Trait,
}

impl Parse for InterfaceLike {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse::<Visibility>()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![impl]) {
            let mut item = ItemImpl::parse(input)?;
            if !matches!(vis, Visibility::Inherited) {
                syn::Error::new_spanned(vis, "unexpected visibility modifier").pipe(Err)
            } else {
                item.attrs = attrs;
                Ok(InterfaceLike::Impl(item))
            }
        } else if lookahead.peek(Token![trait]) {
            let mut item = ItemTrait::parse(input)?;
            item.attrs = attrs;
            item.vis = vis;
            Ok(InterfaceLike::Trait(item))
        } else {
            Err(lookahead.error())
        }
    }
}
