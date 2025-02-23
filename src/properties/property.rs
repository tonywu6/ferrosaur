use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Generics, ReturnType, Signature};
use tap::Pipe;

use crate::util::{FlagName, NewtypeMeta, NonFatalErrors, TypeCast};

use super::{name_or_symbol, property_key, self_arg, MaybeAsync, Property};

pub fn impl_property(prop: Property, sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Error::accumulator();

    MaybeAsync::Sync
        .only::<Property>(&sig)
        .non_fatal(&mut errors);

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

    let return_ty = TypeCast::from(output);

    let name = name_or_symbol::<Property>(ident.span(), name.into_inner(), symbol.into_inner())
        .non_fatal(&mut errors);

    let name = property_key(&ident, name);

    let getter = {
        let getter = return_ty.to_getter(&name);
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
                let this = AsRef::<v8::Global<_>>::as_ref(self);
                let this = v8::Local::new(scope, this);
                getter(scope, this).context(#err)
            }
        }
    };

    let setter = if with_setter.is_present() {
        let ident = format_ident!("set_{}", ident);
        let setter = return_ty.to_setter(&name);
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
                let this = AsRef::<v8::Global<_>>::as_ref(self);
                let this = v8::Local::new(scope, this);
                setter(scope, this, data).context(#err)?;
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
