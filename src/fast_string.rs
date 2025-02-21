use proc_macro2::TokenStream;
use quote::quote;
use syn::LitStr;

pub fn unsafe_include_fast_string(path: LitStr) -> TokenStream {
    quote! {{
        #[cfg(debug_assertions)]
        {
            use deno_core::{v8, FastStaticString};
            const BUFFER: &str = include_str!(#path);
            const STRING: v8::OneByteConst =
                unsafe { v8::String::create_external_onebyte_const_unchecked(BUFFER.as_bytes()) };
            FastStaticString::new(&STRING)
        }
        #[cfg(not(debug_assertions))]
        #[allow(long_running_const_eval)]
        {
            use deno_core::ascii_str_include;
            ascii_str_include!(#path)
        }
    }}
}
