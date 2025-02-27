use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Generics, ReturnType, Signature};
use tap::Pipe;

use crate::util::{
    expect_self_arg, only_pat_ident, FatalErrors, FunctionIntent, RecoverableErrors, V8Conv,
};

use super::Setter;

pub fn impl_setter(_: Setter, sig: Signature) -> Result<Vec<TokenStream>> {
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

    errors.handle(if matches!(output, ReturnType::Default) {
        Ok(())
    } else {
        Error::custom("indexing setters must not have a return type")
            .with_span(&output)
            .pipe(Err)
    });

    let (_, mut errors) = if inputs.len() != 3 {
        Error::custom("require exactly two argument after &self")
            .with_span(&ident)
            .pipe(Err)
    } else {
        Ok(())
    }
    .or_fatal(errors)?;

    let key_name = errors.handle(only_pat_ident(&inputs[1]));
    let key_type = V8Conv::from_fn_arg(inputs[1].clone()).and_recover(&mut errors);

    let val_name = errors.handle(only_pat_ident(&inputs[2]));
    let val_type = V8Conv::from_fn_arg(inputs[2].clone()).and_recover(&mut errors);

    let setter = {
        let setter = val_type.to_setter();
        let from_key = key_type.to_cast_into_v8(
            key_name.map(ToString::to_string).unwrap_or_default(),
            "scope",
        );
        let key_type = key_type.to_type();
        let val_type = val_type.to_type();
        quote! {
            fn #ident <#params> (
                #self_arg,
                #key_name: #key_type,
                #val_name: #val_type,
                _rt: &mut JsRuntime,
            ) -> Result<&Self>
            #where_clause
            {
                #setter
                let scope = &mut _rt.handle_scope();
                let this = ToV8::to_v8(self, scope)?;
                let this = v8::Local::new(scope, this);
                let prop = #from_key?;
                setter(scope, this, prop, #val_name)
                    .context("failed to set property")?;
                Ok(self)
            }
        }
    };

    errors.finish_with(vec![setter])
}
