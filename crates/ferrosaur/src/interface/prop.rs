use darling::{Error, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Signature;
use tap::Pipe;

use crate::util::{
    expect_self_arg, function::FunctionIntent, only_explicit_return_type, v8::V8Conv, NewtypeMeta,
    RecoverableErrors,
};

use super::{Property, ResolveName};

pub fn impl_property(prop: Property, sig: Signature) -> Result<Vec<TokenStream>> {
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

    errors.handle(if inputs.len() > 1 {
        Error::custom("must not have extra arguments")
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

    let name = ResolveName {
        ident: &ident,
        name: name.into_inner(),
        symbol: symbol.into_inner(),
    }
    .resolve()
    .and_recover(&mut errors);

    let return_ty = V8Conv::from_output(output).and_recover(&mut errors);

    let getter = {
        let getter = return_ty.to_getter(&generics);
        let return_ty = return_ty.to_type();
        let err = format!("failed to get property {name:?}");
        let params = &generics.params;
        let where_ = &generics.where_clause;
        quote! {
            fn #ident <#params> (
                #self_arg,
                rt: &mut JsRuntime,
            ) -> Result<#return_ty>
            #where_
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
        let setter = return_ty.to_setter(&generics);
        let data_type = return_ty.to_type();
        let err = format!("failed to set property {name:?}");
        let params = &generics.params;
        let where_ = &generics.where_clause;
        quote! {
            fn #ident <#params> (
                #self_arg,
                data: #data_type,
                _rt: &mut JsRuntime,
            ) -> Result<&Self>
            #where_
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
