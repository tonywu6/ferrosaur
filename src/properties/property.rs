use darling::{
    util::{Flag, SpannedValue},
    Error, FromMeta, Result,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Generics, ReturnType, Signature};
use tap::Pipe;

use crate::properties::setter;

use super::{fn_color, getter, property_key, return_type, self_arg, FnColor, TypeCast};

#[derive(Debug, Default, Clone, FromMeta)]
pub struct Property {
    name: Option<SpannedValue<String>>,
    with_setter: Flag,
    #[darling(default)]
    cast: TypeCast,
}

pub fn impl_property(prop: Property, sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Error::accumulator();

    errors.handle(fn_color(&sig, FnColor::Sync));

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

    let self_arg = errors.handle(self_arg(&inputs, span));

    errors.handle(if inputs.len() > 1 {
        Error::custom("#[property] fn must not have extra arguments")
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

    errors.handle(if matches!(output, ReturnType::Default) {
        Error::custom("#[property] fn must have an explicit return type")
            .with_span(&ident)
            .pipe(Err)
    } else {
        Ok(())
    });

    let return_ty = return_type(&output, cast, &mut errors);

    let prop = property_key(&ident, &name);

    let getter = {
        let getter = getter(&prop, &return_ty, cast);
        let err = format!("failed to get property {:?}", prop.as_str());
        quote! {
            fn #ident <#params> (
                #self_arg,
                scope: &mut v8::HandleScope
            ) -> Result<#return_ty>
            #where_clause
            {
                #getter
                let this = AsRef::<v8::Global<_>>::as_ref(self);
                let this = v8::Local::new(scope, this);
                getter(scope, this).context(#err)
            }
        }
    };

    let setter = if with_setter.is_present() {
        let ident = format_ident!("set_{}", ident);
        let data = quote! { &#return_ty };
        let setter = setter(&prop, &data, cast);
        let err = format!("failed to set property {:?}", prop.as_str());
        quote! {
            fn #ident <#params> (
                #self_arg,
                scope: &mut v8::HandleScope,
                data: #data,
            ) -> Result<&Self>
            #where_clause
            {
                #setter
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
