#![cfg_attr(doctest, doc = include_str!("../../../../docs/src/reference/typing.md"))]

use darling::{Error, FromMeta, Result};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Paren,
    AngleBracketedGenericArguments, BoundLifetimes, FnArg, GenericArgument, GenericParam, Generics,
    Ident, Lifetime, LifetimeParam, Meta, Path, PathArguments, PathSegment, PredicateType,
    ReturnType, Token, TraitBound, TraitBoundModifier, Type, TypeParamBound, TypePath,
    TypeReference, TypeTuple, WherePredicate,
};
use tap::{Pipe, Tap};

use super::{type_ident, unwrap_v8_local, Caveat, MergeGenerics};

#[derive(Debug, Clone)]
pub enum V8Conv {
    Trait { ty: Type },
    Value { ty: Type },
    Serde { ty: Type, hint: Ident },
}

impl V8Conv {
    pub fn from_type(mut ty: Type) -> Caveat<Self> {
        if let UseSerde::Some(hint) = fold_type_hint(&mut ty) {
            (Self::Serde { ty, hint }, None)
        } else if let Caveat(true, err) = has_v8_global(&ty) {
            (Self::Value { ty }, err)
        } else {
            (Self::Trait { ty }, None)
        }
        .into()
    }

    pub fn from_fn_arg(arg: FnArg) -> Caveat<Self> {
        match arg {
            FnArg::Receiver(recv) => Self::from_type(type_ident(Token![Self](recv.span()).into())),
            FnArg::Typed(ty) => Self::from_type(*ty.ty),
        }
    }

    pub fn from_output(ty: ReturnType) -> Caveat<Self> {
        match ty {
            ReturnType::Default => TypeTuple {
                paren_token: Paren(Span::call_site()),
                elems: Punctuated::new(),
            }
            .pipe(Type::Tuple)
            .pipe(|ty| Self::Serde {
                ty,
                hint: format_ident!("serde"),
            })
            .into(),
            ReturnType::Type(_, ty) => Self::from_type(*ty),
        }
    }

    pub fn as_type(&self) -> &Type {
        match self {
            Self::Trait { ty } => ty,
            Self::Value { ty } => ty,
            Self::Serde { ty, .. } => ty,
        }
    }

    pub fn to_type(&self) -> Type {
        self.as_type().clone()
    }

    pub fn to_inner_type(&self) -> Option<Type> {
        let Self::Value {
            ty:
                Type::Path(TypePath {
                    qself: None,
                    path:
                        Path {
                            leading_colon: None,
                            segments,
                        },
                }),
        } = self
        else {
            return None;
        };
        let PathArguments::AngleBracketed(args) = &segments.get(1)?.arguments else {
            return None;
        };
        if args.args.len() == 1 {
            if let GenericArgument::Type(ty) = &args.args[0] {
                Some(ty.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn to_cast_into_v8<K>(&self, name: K, scope: &'static str) -> TokenStream
    where
        K: AsRef<str>,
    {
        let ident = format_ident!("{}", name.as_ref());
        let handle = format_ident!("{scope}");
        match self {
            Self::Trait { .. } => quote! {{
                match ToV8::to_v8(#ident, #handle) {
                    Err(err) => Err(err).context("failed to convert using ToV8"),
                    Ok(data) => match data.try_cast() {
                        Err(err) => Err(err).context("failed to cast into v8 type"),
                        Ok(data) => Ok(data),
                    }
                }
            }},
            Self::Value { .. } => quote! {{
                v8::Local::new(#handle, #ident).try_cast()
            }},
            Self::Serde { hint, .. } => {
                let serde_v8 = Self::mention_serde_v8(hint);
                quote! {{
                    #serde_v8::to_v8(#handle, #ident)
                }}
            }
        }
    }

    pub fn to_cast_from_v8<K>(&self, name: K, scope: &'static str) -> TokenStream
    where
        K: AsRef<str>,
    {
        let ident = format_ident!("{}", name.as_ref());
        let handle = format_ident!("{scope}");
        match self {
            Self::Trait { .. } => quote! {{
                FromV8::from_v8(#handle, #ident.cast::<v8::Value>())
            }},
            Self::Value { .. } => quote! {{
                match #ident.try_cast() {
                    Err(err) => Err(err),
                    Ok(data) => Ok(v8::Global::new(#handle, data)),
                }
            }},
            Self::Serde { hint, .. } => {
                let serde_v8 = Self::mention_serde_v8(hint);
                quote! {{
                    #serde_v8::from_v8(#handle, #ident)
                }}
            }
        }
    }

    pub fn to_getter(&self, generics: &Generics) -> TokenStream {
        let unwrap_data = unwrap_v8_local("data");
        let from_data = self.to_cast_from_v8("data", "scope");
        let return_ty = self.to_type();
        let generics = Self::generics(generics);
        let params = generics.params();
        let bounds = generics.bounds();
        quote! {
            #[inline(always)]
            fn getter<#params>(
                scope: &mut v8::HandleScope<'_a>,
                this: _T,
                prop: _K,
            ) -> Result<#return_ty>
            where
                #bounds
            {
                let scope = &mut v8::TryCatch::new(scope);
                let this = TryInto::try_into(this)
                    .context("failed to cast `self` as a v8::Object")?;
                let prop = Into::into(prop);
                let data = this.get(scope, prop);
                let data = #unwrap_data;
                Ok(#from_data?)
            }
        }
    }

    pub fn to_setter(&self, generics: &Generics) -> TokenStream {
        let into_data = self.to_cast_into_v8("data", "scope");
        let data_type = self.to_type();
        let generics = Self::generics(generics);
        let params = generics.params();
        let bounds = generics.bounds();
        quote! {
            #[inline(always)]
            fn setter<#params>(
                scope: &mut v8::HandleScope<'_a>,
                this: _T,
                prop: _K,
                data: #data_type
            ) -> Result<()>
            where
                #bounds
            {
                let data = #into_data
                    .context("failed to convert into v8 value")?;
                let this = TryInto::try_into(this)
                    .context("failed to cast `self` as a v8::Object")?;
                let prop = Into::into(prop);
                this.set(scope, prop, data);
                Ok(())
            }
        }
    }

    fn generics(outer: &Generics) -> MergeGenerics {
        MergeGenerics {
            outer,
            lifetimes: vec![quote! { '_a }],
            types: vec![quote! { _K }, quote! { _T }],
            bounds: vec![
                quote! { _K: Into<v8::Local<'_a, v8::Value>> },
                quote! { _T: TryInto<v8::Local<'_a, v8::Object>> },
                quote! { _T::Error: ::core::error::Error + Send + Sync + 'static },
            ],
        }
    }

    pub fn new_v8(ty: &str) -> Self {
        [format_ident!("v8"), format_ident!("{}", ty)]
            .map(PathSegment::from)
            .pipe(Punctuated::<PathSegment, Token![::]>::from_iter)
            .pipe(|segments| Path {
                segments,
                leading_colon: None,
            })
            .pipe(|path| TypePath { path, qself: None })
            .pipe(Type::Path)
            .pipe(GenericArgument::Type)
            .pipe(std::iter::once)
            .pipe(Punctuated::<GenericArgument, Token![,]>::from_iter)
            .pipe(|args| AngleBracketedGenericArguments {
                args,
                colon2_token: None,
                lt_token: Token![<](Span::call_site()),
                gt_token: Token![>](Span::call_site()),
            })
            .pipe(PathArguments::AngleBracketed)
            .pipe(|arguments| PathSegment {
                ident: format_ident!("Global"),
                arguments,
            })
            .pipe(|global| [format_ident!("v8").into(), global])
            .pipe(Punctuated::<PathSegment, Token![::]>::from_iter)
            .pipe(|segments| Path {
                segments,
                leading_colon: None,
            })
            .pipe(|path| TypePath { path, qself: None })
            .pipe(Type::Path)
            .pipe(|ty| Self::Value { ty })
    }

    fn mention_serde_v8(hint: &Ident) -> Ident {
        Ident::new("serde_v8", hint.span())
    }
}

impl Default for V8Conv {
    fn default() -> Self {
        Self::new_v8("Value")
    }
}

impl ToTokens for V8Conv {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.as_type().to_tokens(tokens);
    }
}

fn has_v8_global(ty: &Type) -> Caveat<bool> {
    fn is_v8_global(ty: &TypePath) -> Caveat<bool> {
        let TypePath {
            qself: None,
            path:
                Path {
                    leading_colon: None,
                    segments,
                },
        } = ty
        else {
            return false.into();
        };
        if segments.len() != 2 {
            return false.into();
        }
        if segments[0].ident == "v8" && segments[0].arguments.is_none() {
            if segments[1].ident == "Global" && !segments[1].arguments.is_empty() {
                true.into()
            } else {
                let err = Error::custom("expected `v8::Global<v8::...>`").with_span(ty);
                (true, err).into()
            }
        } else {
            false.into()
        }
    }
    match ty {
        Type::Array(inner) => has_v8_global(&inner.elem),
        Type::BareFn(_) => false.into(),
        Type::Group(inner) => has_v8_global(&inner.elem),
        Type::ImplTrait(_) => false.into(),
        Type::Infer(_) => false.into(),
        Type::Macro(_) => false.into(),
        Type::Never(_) => false.into(),
        Type::Paren(inner) => has_v8_global(&inner.elem),
        Type::Path(ty) => is_v8_global(ty),
        Type::Ptr(_) => false.into(),
        Type::Reference(inner) => has_v8_global(&inner.elem),
        Type::Slice(inner) => has_v8_global(&inner.elem),
        Type::TraitObject(_) => false.into(),
        Type::Tuple(inner) => inner.elems.iter().any(|t| has_v8_global(t).0).into(),
        Type::Verbatim(_) => false.into(),
        _ => false.into(),
    }
}

#[derive(Debug, Default, Clone)]
enum UseSerde {
    #[default]
    None,
    Some(Ident),
}

impl TypeHint for UseSerde {
    const HINT: &'static str = "serde";

    fn on_type_hint(&mut self, ident: &Ident) {
        *self = Self::Some(ident.clone())
    }
}

trait TypeHint: Default {
    const HINT: &'static str;
    fn on_type_hint(&mut self, ident: &Ident);
}

fn fold_type_hint<T: TypeHint>(ty: &mut Type) -> T {
    let mut hint = Default::default();
    visit_type_hint(&mut hint, ty);
    hint
}

fn visit_type_hint<T: TypeHint>(hint: &mut T, ty: &mut Type) {
    if !matches!(ty, Type::Path(_)) {
        return match ty {
            Type::Path(_) => unreachable!(),

            Type::Array(inner) => visit_type_hint(hint, &mut inner.elem),
            Type::Group(inner) => visit_type_hint(hint, &mut inner.elem),
            Type::Paren(inner) => visit_type_hint(hint, &mut inner.elem),
            Type::Reference(inner) => visit_type_hint(hint, &mut inner.elem),
            Type::Slice(inner) => visit_type_hint(hint, &mut inner.elem),

            Type::Tuple(inner) => inner
                .elems
                .iter_mut()
                .for_each(|ty| visit_type_hint(hint, ty)),

            Type::BareFn(_) => {}
            Type::ImplTrait(_) => {}
            Type::Infer(_) => {}
            Type::Macro(_) => {}
            Type::Never(_) => {}
            Type::Ptr(_) => {}
            Type::TraitObject(_) => {}
            Type::Verbatim(_) => {}
            _ => {}
        };
    };

    if let Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    }) = ty
    {
        if segments.len() == 1 && segments[0].ident == T::HINT {
            if let Some(inner) = fold_path(&mut segments[0]) {
                hint.on_type_hint(&segments[0].ident);
                *ty = inner;
            }
        } else {
            segments
                .iter_mut()
                .for_each(|item| visit_argument(hint, item));
        }
    }

    fn visit_argument<T: TypeHint>(hint: &mut T, item: &mut PathSegment) {
        if let PathArguments::AngleBracketed(ref mut args) = item.arguments {
            args.args.iter_mut().for_each(|arg| {
                if let GenericArgument::Type(ref mut ty) = arg {
                    visit_type_hint(hint, ty);
                }
            });
        }
    }

    fn fold_path(ty: &mut PathSegment) -> Option<Type> {
        let PathArguments::AngleBracketed(ref mut args) = ty.arguments else {
            return None;
        };
        if args.args.len() != 1 {
            return None;
        }
        let items = std::mem::take(&mut args.args);
        if let Some(GenericArgument::Type(ty)) = items.into_iter().next() {
            Some(ty)
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct V8InnerType(pub V8Conv);

impl FromMeta for V8InnerType {
    fn from_meta(item: &Meta) -> Result<Self> {
        let Meta::Path(path) = item else {
            return Err(Error::custom("expected a type path"));
        };
        quote! { v8::Global<#path> }
            .pipe(|tokens| Type::parse.parse2(tokens))?
            .pipe(V8Conv::from_type)
            .into_result()?
            .pipe(Self)
            .pipe(Ok)
    }
}

pub fn to_v8_bound(self_ty: Type) -> WherePredicate {
    let a = Lifetime::new("'_v8", Span::call_site());

    let for_a = LifetimeParam {
        lifetime: a.clone(),
        attrs: Default::default(),
        colon_token: None,
        bounds: Default::default(),
    }
    .pipe(GenericParam::Lifetime)
    .pipe(|a| BoundLifetimes::default().tap_mut(|b| b.lifetimes.push(a)))
    .pipe(Some);

    let ref_self = TypeReference {
        and_token: Token![&](Span::call_site()),
        lifetime: Some(a.clone()),
        elem: Box::new(self_ty),
        mutability: None,
    }
    .pipe(Type::Reference);

    let to_v8 = PathSegment {
        ident: format_ident!("ToV8"),
        arguments: AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Token![<](Span::call_site()),
            args: std::iter::once(GenericArgument::Lifetime(a)).collect(),
            gt_token: Token![>](Span::call_site()),
        }
        .pipe(PathArguments::AngleBracketed),
    }
    .pipe(|path| Path {
        leading_colon: None,
        segments: std::iter::once(path).collect(),
    })
    .pipe(|path| TraitBound {
        paren_token: None,
        modifier: TraitBoundModifier::None,
        lifetimes: None,
        path,
    })
    .pipe(TypeParamBound::Trait)
    .pipe(std::iter::once)
    .collect();

    PredicateType {
        lifetimes: for_a,
        bounded_ty: ref_self,
        colon_token: Token![:](Span::call_site()),
        bounds: to_v8,
    }
    .pipe(WherePredicate::Type)
}

pub mod snippets {
    use proc_macro2::TokenStream;
    use quote::{quote, ToTokens};

    pub fn impl_from_inner<K: ToTokens>(v8_outer: &TokenStream, ident: K) -> TokenStream {
        quote! {
            #[automatically_derived]
            impl From<#v8_outer> for #ident {
                fn from(value: #v8_outer) -> Self {
                    Self(value)
                }
            }
        }
    }

    pub fn impl_into_inner<K: ToTokens>(v8_outer: &TokenStream, ident: K) -> TokenStream {
        quote! {
            #[automatically_derived]
            impl From<#ident> for #v8_outer {
                fn from(value: #ident) -> Self {
                    value.0
                }
            }
        }
    }

    pub fn impl_as_ref_inner<K: ToTokens>(v8_outer: &TokenStream, ident: K) -> TokenStream {
        quote! {
            #[automatically_derived]
            impl AsRef<#v8_outer> for #ident {
                fn as_ref(&self) -> &#v8_outer {
                    &self.0
                }
            }
        }
    }

    pub fn impl_from_v8<K: ToTokens>(v8_inner: &TokenStream, ident: K) -> TokenStream {
        quote! {
            #[automatically_derived]
            impl<'a> FromV8<'a> for #ident {
                type Error =
                    <v8::Local<'a, v8::Value> as TryInto<v8::Local<'a, #v8_inner>>>::Error;

                fn from_v8(
                    scope: &mut v8::HandleScope<'a>,
                    value: v8::Local<'a, v8::Value>,
                ) -> ::core::result::Result<Self, Self::Error> {
                    Ok(Self(v8::Global::new(scope, value.try_cast()?)))
                }
            }
        }
    }

    pub fn impl_to_v8<K: ToTokens>(v8_inner: &TokenStream, ident: K) -> TokenStream {
        let error = quote! {
            type Error = <v8::Local<'a, #v8_inner> as TryInto<v8::Local<'a, v8::Value>>>::Error;
        };

        let to_v8 = quote! {
            fn to_v8(
                self,
                scope: &mut v8::HandleScope<'a>,
            ) -> ::core::result::Result<v8::Local<'a, v8::Value>, Self::Error> {
                v8::Local::new(scope, &self.0).try_cast()
            }
        };

        quote! {
            #[automatically_derived]
            impl<'a> ToV8<'a> for #ident {
                #error
                #to_v8
            }

            #[automatically_derived]
            impl<'a> ToV8<'a> for &'_ #ident {
                #error
                #to_v8
            }
        }
    }

    pub fn impl_global_cast(v8_inner: &TokenStream) -> TokenStream {
        quote! {
            #[inline(always)]
            pub fn try_cast_global<'a, T>(
                &self,
                rt: &'a mut JsRuntime,
            ) -> ::core::result::Result<
                 v8::Global<T>,
                <v8::Local<'a, #v8_inner> as TryInto<v8::Local<'a, T>>>::Error
            >
            where
                 v8::Local<'a, #v8_inner>:   TryInto<v8::Local<'a, T>>,
            {
                let scope = &mut rt.handle_scope();
                let this = v8::Local::new(scope, &self.0);
                let this = this.try_cast()?;
                Ok(v8::Global::new(scope, this))
            }
        }
    }
}
