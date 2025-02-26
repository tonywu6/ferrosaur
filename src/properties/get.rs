use darling::{error::Accumulator, Result};
use proc_macro2::TokenStream;
use syn::{spanned::Spanned, FnArg, Generics, Signature};

use crate::{
    properties::{self_arg, MaybeAsync},
    util::RecoverableErrors,
};

use super::Getter;

pub fn impl_getter(_: Getter, sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Accumulator::default();

    MaybeAsync::Sync
        .only::<Getter>(&sig)
        .and_recover(&mut errors);

    let span = sig.span();

    let Signature {
        ident,
        generics,
        inputs,
        output,
        ..
    } = sig;

    let Generics {
        params,
        where_clause,
        ..
    } = generics;

    let self_arg = errors.handle(self_arg::<Getter>(&inputs, span));

    todo!()
}
