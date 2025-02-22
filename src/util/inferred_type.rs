use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    punctuated::Punctuated, token::Paren, AngleBracketedGenericArguments, GenericArgument, Ident,
    Path, PathArguments, PathSegment, ReturnType, Token, Type, TypePath, TypeTuple,
};
use tap::{Pipe, Tap};

use super::{unwrap_v8_local, PropertyKey};

#[derive(Debug, Clone)]
pub enum InferredType {
    Serde {
        ty: Type,
    },
    V8 {
        ty: Type,
        newtype: Option<Ident>,
        nullish: bool,
    },
}

impl From<Type> for InferredType {
    fn from(mut ty: Type) -> Self {
        let use_v8 = UseV8::visit(&mut ty);

        fn serde_or_v8(use_v8: UseV8, ty: Type) -> InferredType {
            match use_v8 {
                UseV8::None => InferredType::Serde { ty },
                UseV8::Some => InferredType::V8 {
                    ty,
                    newtype: None,
                    nullish: false,
                },
                UseV8::NewType(newtype) => InferredType::V8 {
                    ty,
                    newtype: Some(newtype),
                    nullish: false,
                },
            }
        }

        let Type::Path(mut ty) = ty else {
            return serde_or_v8(use_v8, ty);
        };

        if ty.path.segments.is_empty() {
            return serde_or_v8(use_v8, Type::Path(ty));
        };

        if ty.path.segments.len() != 1
            || !ty.path.segments.is_empty() && ty.path.segments[0].ident != "Option"
        {
            return serde_or_v8(use_v8, Type::Path(ty));
        }

        let Some(SingleTypeParam { args, ty: inner }) = single_type_param(&mut ty.path.segments[0])
        else {
            return serde_or_v8(use_v8, Type::Path(ty));
        };

        match serde_or_v8(use_v8, Type::Path(inner)) {
            Self::Serde { ty: inner } => {
                args.args.push(GenericArgument::Type(inner));
                Self::Serde { ty: Type::Path(ty) }
            }
            Self::V8 {
                ty: inner, newtype, ..
            } => {
                args.args.push(GenericArgument::Type(inner));
                Self::V8 {
                    ty: Type::Path(ty),
                    newtype,
                    nullish: true,
                }
            }
        }
    }
}

impl From<ReturnType> for InferredType {
    fn from(value: ReturnType) -> Self {
        match value {
            ReturnType::Default => TypeTuple {
                paren_token: Paren(Span::call_site()),
                elems: Punctuated::new(),
            }
            .pipe(Type::Tuple)
            .pipe(Self::from),
            ReturnType::Type(_, ty) => Self::from(*ty),
        }
    }
}

impl InferredType {
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
            .pipe(|ty| Self::V8 {
                ty,
                newtype: None,
                nullish: false,
            })
    }

    pub fn non_null(self) -> Self {
        match self {
            Self::V8 {
                nullish: true,
                ty,
                newtype,
            } => Self::V8 {
                ty,
                newtype,
                nullish: false,
            },
            Self::V8 { nullish: false, .. } => self,
            Self::Serde { .. } => self,
        }
    }

    pub fn to_type(&self) -> Type {
        match self {
            Self::Serde { ty } => ty.clone(),
            Self::V8 { ty, .. } => ty.clone(),
        }
    }

    pub fn to_cast_from_v8<K>(&self, name: K, scope: &'static str) -> TokenStream
    where
        K: AsRef<str>,
    {
        let ident = format_ident!("{}", name.as_ref());
        let handle = format_ident!("{scope}");
        match self {
            Self::V8 { nullish, .. } => {
                let v8 = self.mention_v8();
                let cast = quote! {{
                    match #ident.try_cast() {
                        Ok(value) => Ok(#v8::Global::new(#handle, value)),
                        Err(error) => Err(error)
                    }
                }};
                if *nullish {
                    quote! {{
                        if #ident.is_null_or_undefined() {
                            Ok(None)
                        } else {
                            #cast.map(Some)
                        }
                    }}
                } else {
                    cast
                }
            }
            Self::Serde { .. } => quote! {{
                serde_v8::from_v8(#handle, #ident)
            }},
        }
    }

    pub fn to_cast_into_v8<K>(&self, name: K, scope: &'static str) -> TokenStream
    where
        K: AsRef<str>,
    {
        let ident = format_ident!("{}", name.as_ref());
        let handle = format_ident!("{scope}");
        match self {
            Self::V8 {
                newtype, nullish, ..
            } => {
                let cast = if newtype.is_some() {
                    let v8 = self.mention_v8();
                    quote! {{
                        let #ident = AsRef::<v8::Global<_>>::as_ref(&#ident);
                        #v8::Local::new(#handle, #ident).try_cast()
                    }}
                } else {
                    quote! {{
                        v8::Local::new(#handle, #ident).try_cast()
                    }}
                };
                if *nullish {
                    quote! {{
                        match #ident {
                            None => v8::null(#handle).try_cast(),
                            Some(#ident) => #cast
                        }
                    }}
                } else {
                    cast
                }
            }
            Self::Serde { .. } => quote! {{
                serde_v8::to_v8(#handle, #ident)
            }},
        }
    }

    pub fn to_result<K>(&self, name: K) -> TokenStream
    where
        K: AsRef<str>,
    {
        let ident = format_ident!("{}", name.as_ref());
        match self {
            Self::V8 { nullish: true, .. } => quote! {
                Ok(#ident.map(Into::into))
            },
            Self::V8 { nullish: false, .. } => quote! {
                Ok(Into::into(#ident))
            },
            Self::Serde { .. } => quote! {
                Ok(#ident)
            },
        }
    }

    pub fn to_getter<K>(&self, prop: &PropertyKey<K>) -> TokenStream
    where
        K: AsRef<str>,
    {
        let unwrap_data = unwrap_v8_local("data");
        let from_data = self.to_cast_from_v8("data", "scope");
        let return_ok = self.to_result("data");
        let return_ty = self.to_type();
        quote! {
            #[inline(always)]
            fn getter<'a, T>(
                scope: &mut v8::HandleScope<'a>,
                this: T,
            ) -> Result<#return_ty>
            where
                T: TryInto<v8::Local<'a, v8::Object>>,
                T::Error: ::std::error::Error + Send + Sync + 'static
            {
                let scope = &mut v8::TryCatch::new(scope);
                let this = TryInto::try_into(this)
                    .context("failed to cast `self` as a v8::Object")?;
                let prop = #prop;
                let prop = Into::into(prop);
                let data = this.get(scope, prop);
                let data = #unwrap_data;
                let data = #from_data
                    .context("failed to convert from v8 value")?;
                #return_ok
            }
        }
    }

    pub fn to_setter<K>(&self, prop: &PropertyKey<K>) -> TokenStream
    where
        K: AsRef<str>,
    {
        let into_data = self.to_cast_into_v8("data", "scope");
        let data_type = self.to_type();
        quote! {
            #[inline(always)]
            fn setter<'a, T>(
                scope: &mut v8::HandleScope<'a>,
                this: T,
                data: #data_type
            ) -> Result<()>
            where
                T: TryInto<v8::Local<'a, v8::Object>>,
                T::Error: ::std::error::Error + Send + Sync + 'static
            {
                let data = #into_data
                    .context("failed to convert into v8 value")?;
                let this = TryInto::try_into(this)
                    .context("failed to cast `self` as a v8::Object")?;
                let prop = #prop;
                let prop = Into::into(prop);
                this.set(scope, prop, data);
                Ok(())
            }
        }
    }

    fn mention_v8(&self) -> TokenStream {
        match self {
            Self::V8 {
                newtype: Some(v8), ..
            } => quote! { #v8 },
            _ => quote! { v8 },
        }
    }
}

#[derive(Debug, Clone)]
enum UseV8 {
    None,
    Some,
    NewType(Ident),
}

impl UseV8 {
    fn merge(&mut self, other: Self) {
        match self {
            Self::None => *self = other,
            Self::Some => {
                if let Self::NewType(_) = other {
                    *self = other
                }
            }
            Self::NewType(_) => {}
        }
    }

    fn visit(ty: &mut Type) -> UseV8 {
        fn flat_map_path_segment(mut item: PathSegment, use_v8: &mut UseV8) -> Vec<PathSegment> {
            fn visit_arguments(item: &mut PathSegment, use_v8: &mut UseV8) {
                if let PathArguments::AngleBracketed(ref mut args) = item.arguments {
                    args.args.iter_mut().for_each(|arg| {
                        if let GenericArgument::Type(ref mut ty) = arg {
                            use_v8.merge(UseV8::visit(ty));
                        }
                    });
                }
            }
            if item.ident == "v8" {
                if let Some(SingleTypeParam { ty, .. }) = single_type_param(&mut item) {
                    use_v8.merge(UseV8::NewType(item.ident.clone()));
                    if let Type::Path(ty) =
                        Type::Path(ty).tap_mut(|ty| use_v8.merge(UseV8::visit(ty)))
                    {
                        ty.path.segments.into_iter().collect()
                    } else {
                        vec![item]
                    }
                } else {
                    use_v8.merge(UseV8::Some);
                    visit_arguments(&mut item, use_v8);
                    vec![item]
                }
            } else {
                visit_arguments(&mut item, use_v8);
                vec![item]
            }
        }

        match ty {
            Type::Array(inner) => Self::visit(&mut inner.elem),
            Type::BareFn(_) => Self::None,
            Type::Group(inner) => Self::visit(&mut inner.elem),
            Type::ImplTrait(_) => Self::None,
            Type::Infer(_) => Self::None,
            Type::Macro(_) => Self::None,
            Type::Never(_) => Self::None,
            Type::Paren(inner) => Self::visit(&mut inner.elem),

            Type::Path(ty) => {
                let mut use_v8 = Self::None;
                ty.path.segments = std::mem::take(&mut ty.path.segments)
                    .into_iter()
                    .flat_map(|item| flat_map_path_segment(item, &mut use_v8))
                    .collect();
                use_v8
            }

            Type::Ptr(_) => Self::None,
            Type::Reference(inner) => Self::visit(&mut inner.elem),
            Type::Slice(inner) => Self::visit(&mut inner.elem),
            Type::TraitObject(_) => Self::None,

            Type::Tuple(inner) => inner
                .elems
                .iter_mut()
                .map(Self::visit)
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .next()
                .unwrap_or(Self::None),

            Type::Verbatim(_) => Self::None,
            _ => Self::None,
        }
    }
}

impl Iterator for UseV8 {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        match std::mem::replace(self, Self::None) {
            Self::None => None,
            value => Some(value),
        }
    }
}

struct SingleTypeParam<'a> {
    args: &'a mut AngleBracketedGenericArguments,
    ty: TypePath,
}

fn single_type_param(ty: &mut PathSegment) -> Option<SingleTypeParam> {
    let PathArguments::AngleBracketed(ref mut args) = ty.arguments else {
        return None;
    };

    if args.args.len() != 1 {
        return None;
    }

    if !matches!(
        args.args[0],
        GenericArgument::Type(Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                ..
            }
        }))
    ) {
        return None;
    }

    let items = std::mem::take(&mut args.args);

    if let Some(GenericArgument::Type(Type::Path(ty))) = items.into_iter().next() {
        Some(SingleTypeParam { args, ty })
    } else {
        None
    }
}
