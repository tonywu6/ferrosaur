use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemImpl};

mod global_this;
mod module;
mod newtype;
mod properties;
mod util;

use crate::util::TokenStreamResult;

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    module::module(attr.into(), &item).or_error().into()
}

#[proc_macro_attribute]
pub fn global_this(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    global_this::global_this(attr.into(), &item)
        .or_error()
        .into()
}

#[proc_macro_attribute]
pub fn newtype(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    newtype::newtype(attr.into(), &item).or_error().into()
}

#[proc_macro_attribute]
pub fn properties(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemImpl);
    properties::properties(attr.into(), item).or_error().into()
}
