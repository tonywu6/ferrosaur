use darling::{Error, Result};
use proc_macro2::TokenStream;
use syn::{spanned::Spanned, Signature, Type};
use tap::Pipe;

use crate::util::{
    expect_self_arg, only_pat_ident, CallFunction, Caveat, FunctionIntent, FunctionThis,
    MergeErrors, NewtypeMeta, PropertyKey, RecoverableErrors,
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

fn func_to_call(
    Function { name, symbol, this }: Function,
    sig: &mut Signature,
) -> Caveat<CallFunction> {
    let mut errors = Error::accumulator();

    let name = ResolveName {
        ident: &sig.ident,
        name: name.into_inner(),
        symbol: symbol.into_inner(),
    }
    .resolve()
    .and_recover(&mut errors);

    let mut call = CallFunction::from_sig(sig).and_recover(&mut errors);

    call.source = name.into();
    call.this = this;

    if matches!(this, FunctionThis::Unbound) {
        let is_this = match sig.inputs.get(1) {
            Some(arg) => {
                if let Ok(name) = only_pat_ident(arg) {
                    name == "this"
                } else {
                    false
                }
            }
            _ => false,
        };
        if !is_this {
            "`this(unbound)` requires an explicit `this` as the first argument"
                .pipe(Error::custom)
                .with_span(&if let Some(arg) = sig.inputs.get(1) {
                    arg.span()
                } else {
                    sig.ident.span()
                })
                .pipe(|e| errors.push(e))
        }
    }

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
