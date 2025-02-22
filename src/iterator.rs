use darling::{error::Accumulator, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, Parser},
    ItemImpl,
};

use crate::{util::FatalErrors, Iterator_};

pub fn iterator(_: Iterator_, item: TokenStream) -> Result<TokenStream> {
    let errors = Accumulator::default();

    let (item, mut errors) = ItemImpl::parse.parse2(item).or_fatal(errors)?;

    errors.finish()?;

    Ok(quote! {})
}
