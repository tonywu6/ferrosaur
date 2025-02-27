use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::Ident;

use super::{unwrap_v8_local, PropertyKey, V8Conv};

#[derive(Debug, Clone)]
pub struct BindFunction {
    pub source: FunctionSource,
    pub this: FunctionThis,
    pub ctor: bool,
    pub length: FunctionLength,
}

#[derive(Debug, Clone)]
pub enum FunctionSource {
    Prop(PropertyKey),
    This,
}

#[derive(Debug, Clone, Copy)]
pub enum FunctionLength {
    Fixed(usize),
    Variadic,
}

#[derive(Debug, Clone, Copy)]
pub enum FunctionThis {
    Self_,
    Undefined { this: Span, undefined: Span },
    Unbound,
}

impl ToTokens for BindFunction {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let func = V8Conv::new_v8("Function");

        let get_func = match &self.source {
            FunctionSource::Prop(prop) => {
                let getter = func.to_getter();
                quote! {{
                    #getter
                    let prop = #prop;
                    getter(scope, object, prop)
                        .context("failed to get function object")?
                }}
            }
            FunctionSource::This => {
                quote! {{
                    object.try_cast::<v8::Function>()
                        .context("failed to cast self as function")?
                }}
            }
        };

        let object_ty = match &self.source {
            FunctionSource::Prop(_) => quote! { v8::Object },
            FunctionSource::This => quote! { v8::Function },
        };

        let get_bind = {
            let getter = func.to_getter();
            let prop = PropertyKey::from("bind");
            quote! {{
                #getter
                let prop = #prop;
                getter(scope, object, prop)
                    .context("failed to get Function.property.bind from function")?
            }}
        };

        let args_ty = match self.length {
            FunctionLength::Fixed(len) => quote! {
                [v8::Global<v8::Value>; #len]
            },
            FunctionLength::Variadic => quote! {
                Vec<v8::Global<v8::Value>>
            },
        };

        let this = match self.this {
            FunctionThis::Self_ => Some(quote! {{
                let object = TryInto::try_into(object)
                    .context("failed to cast `self` as a v8::Object")?;
                object.cast::<v8::Value>()
            }}),
            FunctionThis::Undefined { this, undefined } => {
                let this = Ident::new("this", this);
                let undefined = Ident::new("undefined", undefined);
                Some(quote! {{
                    let #this = v8::#undefined(scope);
                    #this.cast::<v8::Value>()
                }})
            }
            FunctionThis::Unbound => None,
        };

        let args = if let Some(this) = this {
            match self.length {
                FunctionLength::Fixed(len) => {
                    let locals = (0..len).map(|idx| {
                        let offset = idx + 1;
                        quote! {
                            array[#offset] = v8::Local::new(scope, &args[#idx]);
                        }
                    });
                    let total_len = len + 1;
                    quote! {{
                        let undefined = v8::undefined(scope).cast::<v8::Value>();
                        let mut array = [undefined; #total_len];
                        let this = #this;
                        array[0] = this;
                        #(#locals)*
                        array
                    }}
                }
                FunctionLength::Variadic => quote! {{
                    let this = #this;
                    ::core::iter::once(this)
                        .chain(args.iter().map(|arg| v8::Local::new(scope, arg)))
                        .collect::<Vec<_>>()
                }},
            }
        } else {
            match self.length {
                FunctionLength::Fixed(_) => quote! {{
                    args.map(|arg| v8::Local::new(scope, arg))
                }},
                FunctionLength::Variadic => quote! {{
                    args.into_iter()
                        .map(|arg| v8::Local::new(scope, arg))
                        .collect::<Vec<_>>()
                }},
            }
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

        let result = quote! {
            #[inline(always)]
            fn call<'a, T>(
                scope: &mut v8::HandleScope<'a>,
                object: v8::Local<'a, T>,
                #[allow(unused)]
                args: #args_ty,
            ) -> Result<v8::Global<v8::Value>>
            where
                v8::Local<'a, T>: TryInto<v8::Local<'a, #object_ty>,
                    Error: ::core::error::Error + Send + Sync + 'static>,
            {
                let func = #get_func;
                let bind = {
                    let object = v8::Local::new(scope, &func);
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
        };

        tokens.extend(result);
    }
}

impl<T> From<T> for FunctionSource
where
    T: Into<PropertyKey>,
{
    fn from(value: T) -> Self {
        Self::Prop(value.into())
    }
}
