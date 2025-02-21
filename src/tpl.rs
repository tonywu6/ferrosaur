use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::ReturnType;

use crate::util::{PropertyKey, TypeCast};

impl TypeCast {
    #[allow(clippy::wrong_self_convention)]
    pub fn from_v8_local<K: AsRef<str>>(&self, name: K, scope: &'static str) -> TokenStream {
        let ident = format_ident!("{}", name.as_ref());
        let handle = format_ident!("{scope}");
        match self {
            TypeCast::Serde => quote! {{
                serde_v8::from_v8(#handle, #ident)
            }},
            TypeCast::V8 => quote! {{
                match #ident.try_cast() {
                    Ok(value) => Ok(v8::Global::new(#handle, value)),
                    Err(error) => Err(error)
                }
            }},
            TypeCast::V8Nullish => {
                let inner = TypeCast::V8.from_v8_local(name.as_ref(), scope);
                quote! {{
                    if #ident.is_null_or_undefined() {
                        Ok(None)
                    } else {
                        #inner.map(Some)
                    }
                }}
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn into_v8_local<K: AsRef<str>>(&self, name: K, scope: &'static str) -> TokenStream {
        let ident = format_ident!("{}", name.as_ref());
        let handle = format_ident!("{scope}");
        match self {
            TypeCast::Serde => quote! {{
                serde_v8::to_v8(#handle, #ident)
            }},
            TypeCast::V8 => quote! {{
                v8::Local::new(#handle, #ident).try_cast()
            }},
            TypeCast::V8Nullish => {
                let inner = TypeCast::V8.into_v8_local(name.as_ref(), scope);
                quote! {{
                    match #ident {
                        None => v8::null(#handle).try_cast(),
                        Some(#ident) => #inner
                    }
                }}
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn into_return_value<K: AsRef<str>>(&self, name: K) -> TokenStream {
        let ident = format_ident!("{}", name.as_ref());
        match self {
            TypeCast::Serde => quote! {
                Ok(#ident)
            },
            TypeCast::V8 => quote! {
                Ok(Into::into(#ident))
            },
            TypeCast::V8Nullish => quote! {
                Ok(#ident.map(Into::into))
            },
        }
    }
}

pub fn getter<K, T>(prop: &PropertyKey<K>, cast: TypeCast, ty: T) -> TokenStream
where
    K: AsRef<str>,
    T: ToTokens,
{
    let unwrap_data = unwrap_v8_local("data");
    let from_data = cast.from_v8_local("data", "scope");
    let return_ok = cast.into_return_value("data");
    quote! {
        #[inline(always)]
        fn getter<'a, T>(
            scope: &mut v8::HandleScope<'a>,
            this: T,
        ) -> Result<#ty>
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

pub fn setter<K, T>(prop: &PropertyKey<K>, cast: TypeCast, ty: T) -> TokenStream
where
    K: AsRef<str>,
    T: ToTokens,
{
    let into_data = cast.into_v8_local("data", "scope");
    quote! {
        #[inline(always)]
        fn setter<'a, T>(
            scope: &mut v8::HandleScope<'a>,
            this: T,
            data: #ty
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

pub fn unwrap_v8_local(name: &str) -> TokenStream {
    let err = format!("{name} is None");
    let name = format_ident!("{name}");
    quote! {{
        let Some(#name) = #name else {
            return if let Some(exception) = scope.exception() {
                Err(JsError::from_v8_exception(scope, exception))?
            } else {
                Err(anyhow!(#err))
            };
        };
        #name
    }}
}

pub fn return_type(ty: &ReturnType) -> TokenStream {
    match ty {
        ReturnType::Type(_, ty) => quote! { #ty },
        ReturnType::Default => quote! { () },
    }
}

#[derive(Debug, Clone)]
pub struct Call {
    pub name: PropertyKey<String>,
    pub ctor: bool,
    pub arity: Arity,
}

#[derive(Debug, Clone, Copy)]
pub enum Arity {
    Fixed(usize),
    Variadic,
}

impl Call {
    pub fn render(&self) -> TokenStream {
        let get_func = {
            let data = quote! { v8::Global<v8::Function> };
            let getter = getter(&self.name, TypeCast::V8, &data);
            quote! {{
                #getter
                getter(scope, this)
                    .context("failed to get function object")?
            }}
        };

        let get_bind = {
            let data = quote! { v8::Global<v8::Function> };
            let getter = getter(&"bind".into(), TypeCast::V8, &data);
            quote! {{
                #getter
                getter(scope, this)
                    .context("failed to get Function.property.bind from function")?
            }}
        };

        let args_ty = match self.arity {
            Arity::Fixed(len) => quote! {
                [v8::Global<v8::Value>; #len]
            },
            Arity::Variadic => quote! {
                Vec<v8::Global<v8::Value>>
            },
        };

        let args = match self.arity {
            Arity::Fixed(_) => quote! {
                args.map(|arg| v8::Local::new(scope, arg))
            },
            Arity::Variadic => quote! {
                args
                    .iter()
                    .map(|arg| v8::Local::new(scope, arg))
                    .collect::<Vec<_>>()
            },
        };

        let unwrap_callable = unwrap_v8_local("callable");

        let retval = if self.ctor {
            quote! {{
                callable.new_instance(scope, &[])
            }}
        } else {
            quote! {{
                let recv = v8::undefined(scope);
                callable.call(scope, recv.into(), &[])
            }}
        };

        let unwrap_retval = unwrap_v8_local("retval");

        quote! {
            #[inline(always)]
            fn call<'a, T>(
                scope: &mut v8::HandleScope<'a>,
                this: T,
                args: #args_ty,
            ) -> Result<v8::Global<v8::Value>>
            where
                T: TryInto<v8::Local<'a, v8::Object>>,
                T::Error: std::error::Error + Send + Sync + 'static,
            {
                let func = #get_func;
                let bind = {
                    let this = v8::Local::new(scope, &func);
                    #get_bind
                };
                let bind = v8::Local::new(scope, bind);
                let func = v8::Local::new(scope, func);
                let func = Into::into(func);
                let args = #args;
                let scope = &mut v8::TryCatch::new(scope);
                let callable = bind.call(scope, func, &args);
                let callable = #unwrap_callable;
                let callable = callable.try_cast::<v8::Function>()?;
                let retval = #retval;
                let retval = #unwrap_retval.try_cast()?;
                Ok(v8::Global::new(scope, retval))
            }
        }
    }
}
