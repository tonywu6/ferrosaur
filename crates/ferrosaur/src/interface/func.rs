use darling::{Error, Result};
use proc_macro2::TokenStream;
use syn::{Signature, Type};
use tap::{Pipe, Tap};

use crate::util::{
    expect_self_arg,
    function::{CallFunction, FunctionIntent},
    property::PropertyKey,
    Caveat, MergeErrors, NewtypeMeta, RecoverableErrors,
};

use super::{property_key, Constructor, Function, ResolveName};

pub enum Callable {
    Func(Function),
    Ctor(Constructor),
}

pub fn impl_function(call: Callable, mut sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Error::accumulator();

    let call = match call {
        Callable::Func(func) => func_to_call(func, &mut sig).and_recover(&mut errors),
        Callable::Ctor(ctor) => ctor_to_call(ctor, &mut sig).and_recover(&mut errors),
    };

    let fn_self = errors.handle(expect_self_arg(&sig.inputs, &sig.ident));

    let impl_ = call.render(fn_self, &sig.ident, &sig.generics);

    errors.finish()?;

    Ok(vec![impl_])
}

impl From<Function> for Callable {
    fn from(value: Function) -> Self {
        Self::Func(value)
    }
}

impl From<Constructor> for Callable {
    fn from(value: Constructor) -> Self {
        Self::Ctor(value)
    }
}

fn func_to_call(Function { name, symbol }: Function, sig: &mut Signature) -> Caveat<CallFunction> {
    let mut errors = Error::accumulator();

    let name = ResolveName {
        ident: &sig.ident,
        name: name.into_inner(),
        symbol: symbol.into_inner(),
    }
    .resolve()
    .and_recover(&mut errors);

    let call = CallFunction::from_sig(sig)
        .and_recover(&mut errors)
        .tap_mut(|call| call.source = name.into());

    (call, errors.into_one()).into()
}

fn ctor_to_call(Constructor { class }: Constructor, sig: &mut Signature) -> Caveat<CallFunction> {
    let mut errors = Error::accumulator();

    let mut call = CallFunction::from_sig(sig).and_recover(&mut errors);

    let name = match (class.into_inner().into_inner(), &call.output) {
        (Some(class), _) => PropertyKey::String(class),

        (None, Some(ty)) => {
            let ty = ty.as_type();
            let ident = match ty {
                Type::Path(ty) => {
                    let last = ty.path.segments.last();
                    if let Some(last) = last {
                        if last.arguments.is_none() {
                            Some(&last.ident)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(ident) = ident {
                if ident == "Self" {
                    "constructor return type cannot be `Self`\nthis will be translated as `new Self(...)` which is likely not what you want"
                        .pipe(Error::custom)
                        .with_span(ty)
                        .pipe(|e| errors.push(e));
                }
                PropertyKey::String(ident.to_string())
            } else {
                "cannot infer class name from return type\nspecify `#[js(new(class(...)))]` instead"
                    .pipe(Error::custom)
                    .with_span(ty)
                    .pipe(|e| errors.push(e));
                property_key(&sig.ident, None)
            }
        }

        (None, None) => {
            "cannot infer class name\nspecify a return type, or use `#[js(new(class(...)))]`"
                .pipe(Error::custom)
                .with_span(&sig.ident)
                .pipe(|e| errors.push(e));
            property_key(&sig.ident, None)
        }
    };

    call.source = name.into();

    call.intent = FunctionIntent::Constructed
        .only(sig)
        .and_recover(&mut errors);

    (call, errors.into_one()).into()
}
