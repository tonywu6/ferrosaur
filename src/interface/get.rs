use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Generics, Signature};
use tap::Pipe;

use crate::util::{
    expect_self_arg, function::FunctionIntent, only_explicit_return_type, only_pat_ident,
    v8::V8Conv, FatalErrors, RecoverableErrors,
};

use super::Getter;

pub fn impl_getter(_: Getter, sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Error::accumulator();

    FunctionIntent::Called.only(&sig).and_recover(&mut errors);

    let Signature {
        ident,
        generics: Generics {
            params,
            where_clause,
            ..
        },
        inputs,
        output,
        ..
    } = sig;

    let self_arg = errors.handle(expect_self_arg(&inputs, &ident));

    errors.handle(only_explicit_return_type(&output, &ident));

    let (_, mut errors) = if inputs.len() != 2 {
        Error::custom("require exactly one argument after &self")
            .with_span(&ident)
            .pipe(Err)
    } else {
        Ok(())
    }
    .or_fatal(errors)?;

    let indexer = errors.handle(only_pat_ident(&inputs[1]));

    let indexer_ty = V8Conv::from_fn_arg(inputs[1].clone()).and_recover(&mut errors);

    let return_ty = V8Conv::from_output(output).and_recover(&mut errors);

    let getter = {
        let getter = return_ty.to_getter();
        let from_index = indexer_ty.to_cast_into_v8(
            indexer.map(ToString::to_string).unwrap_or_default(),
            "scope",
        );
        let indexer_ty = indexer_ty.as_type();
        let return_ty = return_ty.to_type();
        quote! {
            fn #ident <#params> (
                #self_arg,
                #indexer: #indexer_ty,
                rt: &mut JsRuntime,
            ) -> Result<#return_ty>
            #where_clause
            {
                #getter
                let scope = &mut rt.handle_scope();
                let this = ToV8::to_v8(self, scope)?;
                let this = v8::Local::new(scope, this);
                let prop = #from_index?;
                getter(scope, this, prop)
                    .context("failed to index into object")
            }
        }
    };

    errors.finish_with(vec![getter])
}
