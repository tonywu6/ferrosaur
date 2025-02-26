use darling::{Error, Result};
use proc_macro2::TokenStream;
use syn::{spanned::Spanned, Signature, Type};
use tap::Pipe;

use crate::util::{
    CallFunction, Caveat, FunctionIntent, MergeErrors, NewtypeMeta, PropertyKey, RecoverableErrors,
};

use super::{name_or_symbol, property_key, self_arg, Constructor, Function};

pub enum Callable {
    Func(Function),
    Ctor(Constructor),
}

pub fn impl_function(call: Callable, mut sig: Signature) -> Result<Vec<TokenStream>> {
    let mut errors = Error::accumulator();

    let fn_self = errors.handle(self_arg(&sig.inputs, sig.span())).cloned();

    let call = match call {
        Callable::Func(func) => func_to_call(func, &mut sig).and_recover(&mut errors),
        Callable::Ctor(ctor) => ctor_to_call(ctor, &mut sig).and_recover(&mut errors),
    };

    let impl_ = call.render(fn_self.as_ref(), &sig.ident, &sig.generics);

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

fn func_to_call(
    Function { name, symbol, this }: Function,
    sig: &mut Signature,
) -> Caveat<CallFunction> {
    let mut errors = Error::accumulator();

    let mut call = CallFunction::from_sig(sig).and_recover(&mut errors);

    let name =
        name_or_symbol(sig.span(), name.into_inner(), symbol.into_inner()).and_recover(&mut errors);
    let name = property_key(&sig.ident, name);

    call.source = name.into();
    call.this = this;

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
