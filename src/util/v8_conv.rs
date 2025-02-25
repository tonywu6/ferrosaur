use darling::{Error, FromMeta, Result};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, Parser},
    punctuated::Punctuated,
    token::Paren,
    AngleBracketedGenericArguments, GenericArgument, Ident, Meta, Path, PathArguments, PathSegment,
    ReturnType, Token, Type, TypePath, TypeTuple,
};
use tap::Pipe;

use super::{unwrap_v8_local, Caveat};

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

    pub fn from_output(ty: ReturnType) -> Caveat<Self> {
        match ty {
            ReturnType::Default => TypeTuple {
                paren_token: Paren(Span::call_site()),
                elems: Punctuated::new(),
            }
            .pipe(Type::Tuple)
            .pipe(Self::from_type),
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

    pub fn to_getter(&self) -> TokenStream {
        let unwrap_data = unwrap_v8_local("data");
        let from_data = self.to_cast_from_v8("data", "scope");
        let return_ty = self.to_type();
        quote! {
            #[inline(always)]
            fn getter<'a, K, T>(
                scope: &mut v8::HandleScope<'a>,
                this: T,
                prop: K,
            ) -> Result<#return_ty>
            where
                K: Into<v8::Local<'a, v8::Value>>,
                T: TryInto<v8::Local<'a, v8::Object>>,
                T::Error: ::std::error::Error + Send + Sync + 'static
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

    pub fn to_setter(&self) -> TokenStream {
        let into_data = self.to_cast_into_v8("data", "scope");
        let data_type = self.to_type();
        quote! {
            #[inline(always)]
            fn setter<'a, K, T>(
                scope: &mut v8::HandleScope<'a>,
                this: T,
                prop: K,
                data: #data_type
            ) -> Result<()>
            where
                K: Into<v8::Local<'a, v8::Value>>,
                T: TryInto<v8::Local<'a, v8::Object>>,
                T::Error: ::std::error::Error + Send + Sync + 'static
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
