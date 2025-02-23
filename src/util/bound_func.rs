use proc_macro2::TokenStream;
use quote::quote;

use super::{unwrap_v8_local, InferredType, PropertyKey};

#[derive(Debug, Clone)]
pub struct BoundFunc {
    pub name: PropertyKey<String>,
    pub ctor: bool,
    pub arity: FuncArity,
}

#[derive(Debug, Clone, Copy)]
pub enum FuncArity {
    Fixed(usize),
    Variadic,
}

impl BoundFunc {
    pub fn to_function(&self) -> TokenStream {
        let func = InferredType::new_v8("Function");

        let get_func = {
            let getter = func.to_getter(&self.name);
            quote! {{
                #getter
                getter(scope, this)
                    .context("failed to get function object")?
            }}
        };

        let get_bind = {
            let getter = func.to_getter(&"bind".into());
            quote! {{
                #getter
                getter(scope, this)
                    .context("failed to get Function.property.bind from function")?
            }}
        };

        let args_ty = match self.arity {
            FuncArity::Fixed(len) => quote! {
                [v8::Global<v8::Value>; #len]
            },
            FuncArity::Variadic => quote! {
                Vec<v8::Global<v8::Value>>
            },
        };

        let args = match self.arity {
            FuncArity::Fixed(_) => quote! {
                args.map(|arg| v8::Local::new(scope, arg))
            },
            FuncArity::Variadic => quote! {
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
