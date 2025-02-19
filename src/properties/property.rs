use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Generics, ReturnType, Signature};
use tap::Pipe;

use crate::util::FeatureName;

use super::{getter, property_key, return_type, self_arg, setter, FnColor, Property};

pub fn impl_property(prop: Property, sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Error::accumulator();

    errors.handle(FnColor::Sync.only::<Property>(&sig));

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
        cast,
        with_setter,
    } = prop;

    let return_ty = return_type(&output);

    errors.handle(if matches!(output, ReturnType::Default) {
        Property::error("fn must have an explicit return type")
            .with_span(&ident)
            .pipe(Err)
    } else {
        Ok(())
    });

    errors.handle(cast.option_check::<Property>(&output));

    let prop = property_key(&ident, &name);

    let getter = {
        let getter = getter(&prop, cast, &return_ty);
        let err = format!("failed to get property {:?}", prop.as_str());
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
        let data = quote! { &#return_ty };
        let setter = setter(&prop, cast, &data);
        let err = format!("failed to set property {:?}", prop.as_str());
        quote! {
            fn #ident <#params> (
                #self_arg,
                data: #data,
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
