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
