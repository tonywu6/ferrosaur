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
        generics,
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

    let key_name = errors.handle(only_pat_ident(&inputs[1]));
    let key_type = V8Conv::from_fn_arg(inputs[1].clone()).and_recover(&mut errors);

    let val_type = V8Conv::from_output(output).and_recover(&mut errors);

    let getter = {
        let getter = val_type.to_getter(&generics);
        let from_key = key_name.map(ToString::to_string).unwrap_or_default();
        let from_key = key_type.to_cast_into_v8(from_key, "scope");
        let key_type = key_type.as_type();
        let val_type = val_type.to_type();

        let Generics {
            params,
            where_clause,
            ..
        } = generics;

        quote! {
            fn #ident <#params> (
                #self_arg,
                #key_name: #key_type,
                rt: &mut JsRuntime,
            ) -> Result<#val_type>
            #where_clause
            {
                #getter
                let scope = &mut rt.handle_scope();
                let this = ToV8::to_v8(self, scope)?;
                let this = v8::Local::new(scope, this);
                let prop = #from_key?;
                getter(scope, this, prop)
                    .context("failed to index into object")
            }
        }
    };

    errors.finish_with(vec![getter])
}
