use darling::{error::Accumulator, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Generics, ReturnType, Signature};
use tap::Pipe;

use crate::util::{FlagName, NewtypeMeta, RecoverableErrors, V8Conv};

use super::{name_or_symbol, property_key, self_arg, MaybeAsync, Property};

pub fn impl_property(prop: Property, sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Accumulator::default();

    MaybeAsync::Sync
        .only::<Property>(&sig)
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

    let self_arg = errors.handle(self_arg::<Property>(&inputs, span));

    errors.handle(if inputs.len() > 1 {
        Property::error("fn must not have extra arguments")
            .with_span(&inputs.get(1))
            .pipe(Err)
    } else {
        Ok(())
    });

    let Property {
        name,
        symbol,
        with_setter,
    } = prop;

    errors.handle(if matches!(output, ReturnType::Default) {
        Property::error("fn must have an explicit return type")
            .with_span(&ident)
            .pipe(Err)
    } else {
        Ok(())
    });

    let return_ty = V8Conv::from_output(output).and_recover(&mut errors);

    let name = name_or_symbol::<Property>(ident.span(), name.into_inner(), symbol.into_inner())
        .and_recover(&mut errors);

    let name = property_key(&ident, name);

    let getter = {
        let getter = return_ty.to_getter();
        let return_ty = return_ty.to_type();
        let err = format!("failed to get property {name:?}");
        quote! {
            fn #ident <#params> (
                #self_arg,
                rt: &mut JsRuntime,
            ) -> Result<#return_ty>
            #where_clause
            {
                #getter
                let scope = &mut rt.handle_scope();
                let this = ToV8::to_v8(self, scope)?;
                let this = v8::Local::new(scope, this);
                let prop = #name;
                getter(scope, this, prop).context(#err)
            }
        }
    };

    let setter = if with_setter.is_present() {
        let ident = format_ident!("set_{}", ident);
        let setter = return_ty.to_setter();
        let data_type = return_ty.to_type();
        let err = format!("failed to set property {name:?}");
        quote! {
            fn #ident <#params> (
                #self_arg,
                data: #data_type,
                _rt: &mut JsRuntime,
            ) -> Result<&Self>
            #where_clause
            {
                #setter
                let scope = &mut _rt.handle_scope();
                let this = ToV8::to_v8(self, scope)?;
                let this = v8::Local::new(scope, this);
                let prop = #name;
                setter(scope, this, prop, data).context(#err)?;
                Ok(self)
            }
        }
    } else {
        quote! {}
    };

    errors.finish()?;

    if with_setter.is_present() {
        Ok(vec![getter, setter])
    } else {
        Ok(vec![getter])
    }
}
