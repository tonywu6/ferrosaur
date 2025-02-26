use darling::Error;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    token::Paren, Expr, FnArg, Generics, Ident, Pat, PatIdent, PatRange, RangeLimits, Receiver,
    ReturnType, Signature, Token, Type, TypeTuple,
};
use tap::{Pipe, Tap};

use super::{
    bind_function::{BindFunction, FunctionLength, FunctionSource, FunctionThis},
    Caveat, MergeErrors, RecoverableErrors, V8Conv,
};

#[derive(Debug, Clone)]
pub struct CallFunction {
    pub intent: FunctionIntent,
    pub source: FunctionSource,
    pub this: FunctionThis,
    pub inputs: Vec<FunctionInput>,
    pub output: Option<V8Conv>,
}

#[derive(Debug, Clone, Copy)]
pub enum FunctionIntent {
    Called,
    Constructed,
    Awaited(Token![async]),
}

#[derive(Debug, Clone)]
pub struct FunctionInput {
    pub ident: Ident,
    pub ty: V8Conv,
    pub spread: bool,
}

impl CallFunction {
    pub fn render(
        &self,
        fn_self: Option<&Receiver>,
        fn_name: &Ident,
        fn_generics: &Generics,
    ) -> TokenStream {
        let Self { intent, .. } = self;

        let asyncness = intent;

        let error_ctx = {
            let dbg_name = match &self.source {
                FunctionSource::Prop(prop) => &format!(" {prop:?}"),
                FunctionSource::This => "",
            };
            match self.intent {
                FunctionIntent::Called | FunctionIntent::Awaited(_) => {
                    format!("failed to call function {dbg_name}")
                }
                FunctionIntent::Constructed => {
                    format!("failed to construct {dbg_name}")
                }
            }
        };

        let (casts, length) = {
            if self
                .inputs
                .iter()
                .any(|FunctionInput { spread, .. }| *spread)
            {
                let casts = self
                    .inputs
                    .iter()
                    .map(|FunctionInput { ident, ty, spread }| {
                        let name = ident.to_string();
                        if *spread {
                            let err = format!("failed to serialize item in argument {name:?}");
                            let var = ty.to_cast_into_v8("arg", "__scope");
                            quote! {
                                #[allow(for_loops_over_fallibles)]
                                for arg in #ident {
                                    let arg = #var.context(#err).context(#error_ctx)?;
                                    let arg = v8::Global::new(__scope, arg);
                                    __args.push(arg);
                                }
                            }
                        } else {
                            let err = format!("failed to serialize argument {name:?}");
                            let var = ty.to_cast_into_v8(&name, "__scope");
                            quote! {
                                let #ident = #var.context(#err).context(#error_ctx)?;
                                let #ident = v8::Global::new(__scope, #ident);
                                __args.push(#ident);
                            }
                        }
                    });

                let casts = quote! {{
                    let __scope = &mut _rt.handle_scope();
                    let mut __args = Vec::new();
                    #(#casts)*
                    __args
                }};

                (casts, FunctionLength::Variadic)
            } else {
                let casts = self.inputs.iter().map(|FunctionInput { ident, ty, .. }| {
                    let name = ident.to_string();
                    let err = format!("failed to serialize argument {name:?}");
                    let var = ty.to_cast_into_v8(&name, "__scope");
                    quote! {
                        let #ident = #var.context(#err).context(#error_ctx)?;
                        let #ident = v8::Global::new(__scope, #ident);
                    }
                });

                let names = self.inputs.iter().map(|FunctionInput { ident, .. }| ident);

                let casts = quote! {{
                    let __scope = &mut _rt.handle_scope();
                    #(#casts)*
                    [#(#names),*]
                }};

                (casts, FunctionLength::Fixed(self.inputs.len()))
            }
        };

        let fn_call = BindFunction {
            source: self.source.clone(),
            this: self.this,
            ctor: matches!(self.intent, FunctionIntent::Constructed),
            length,
        };

        let resolve_output = match self.intent {
            FunctionIntent::Awaited(_) => quote! {{
                let future = _rt.resolve(output);
                _rt.with_event_loop_promise(future, Default::default())
                    .await?
            }},
            FunctionIntent::Called | FunctionIntent::Constructed => quote! {
                output
            },
        };

        let into_output = match &self.output {
            None => {
                quote! {{
                    let _ = output;
                    Ok(())
                }}
            }
            Some(ty) => {
                let from_output = ty.to_cast_from_v8("output", "scope");
                quote! {{
                    let scope = &mut _rt.handle_scope();
                    let output = v8::Local::new(scope, output);
                    let output = #from_output
                        .context("failed to convert returned value")
                        .context(#error_ctx)?;
                    Ok(output)
                }}
            }
        };

        let return_ty = match &self.output {
            None => Type::Tuple(TypeTuple {
                paren_token: Paren(Span::call_site()),
                elems: Default::default(),
            }),
            Some(ty) => ty.to_type(),
        };

        let inputs = &self.inputs;

        let Generics {
            params,
            where_clause,
            ..
        } = fn_generics;

        quote! {
            #asyncness fn #fn_name <#params> (
                #fn_self,
                #(#inputs,)*
                _rt: &mut JsRuntime,
            ) -> Result<#return_ty>
            #where_clause
            {
                let args = #casts;

                let output = {
                    #fn_call
                    let scope = &mut _rt.handle_scope();
                    let object = ToV8::to_v8(self, scope)?;
                    let object = v8::Local::new(scope, object);
                    call(scope, object, args)
                        .context(#error_ctx)?
                };

                let output = #resolve_output;
                #into_output
            }
        }
    }
}

impl CallFunction {
    pub fn from_sig(sig: &mut Signature) -> Caveat<Self> {
        let mut errors = Error::accumulator();

        let mut inputs = Vec::<FunctionInput>::new();

        sig.inputs = std::mem::take(&mut sig.inputs)
            .into_iter()
            .skip(1)
            .map(|arg| {
                let FnArg::Typed(arg) = arg else { return arg };

                let ty = V8Conv::from_type((*arg.ty).clone()).and_recover(&mut errors);

                match *arg.pat {
                    Pat::Ident(ref ident) => {
                        if let Some((sub, _)) = &ident.subpat {
                            Error::custom("subpattern not supported\nremove this")
                                .with_span(sub)
                                .pipe(|e| errors.push(e));
                        }
                        if let Some(ref_) = &ident.by_ref {
                            Error::custom("`ref` not supported\nremove this")
                                .with_span(ref_)
                                .pipe(|e| errors.push(e));
                        }
                        if let Some(mut_) = &ident.mutability {
                            Error::custom("`mut` not supported\nremove this")
                                .with_span(mut_)
                                .pipe(|e| errors.push(e));
                        }
                        let spread = false;
                        let ident = ident.ident.clone();
                        let arg = arg.tap_mut(|arg| arg.ty = ty.to_type().into());
                        inputs.push(FunctionInput { ident, ty, spread });
                        FnArg::Typed(arg)
                    }

                    Pat::Range(PatRange {
                        start: None,
                        end: Some(ref end),
                        limits: RangeLimits::HalfOpen(..),
                        ..
                    }) => match &**end {
                        Expr::Path(path)
                            if path.path.segments.len() == 1
                                && path.path.leading_colon.is_none()
                                && path.qself.is_none() =>
                        {
                            let spread = true;
                            let ident = path.path.segments[0].ident.clone();
                            let arg = arg.tap_mut(|arg| {
                                arg.pat = Pat::Ident(PatIdent {
                                    attrs: vec![],
                                    by_ref: None,
                                    mutability: None,
                                    ident: ident.clone(),
                                    subpat: None,
                                })
                                .into();
                                arg.ty = ty.to_type().into();
                            });
                            inputs.push(FunctionInput { ident, ty, spread });
                            FnArg::Typed(arg)
                        }
                        expr => {
                            "spread argument should be written as `..name`\nfound extra patterns"
                                .pipe(Error::custom)
                                .with_span(expr)
                                .pipe(|e| errors.push(e));
                            FnArg::Typed(arg)
                        }
                    },

                    Pat::Range(PatRange {
                        start: Some(ref start),
                        ..
                    }) => {
                        "spread argument should be written as `..name`"
                            .pipe(Error::custom)
                            .with_span(start)
                            .pipe(|e| errors.push(e));
                        FnArg::Typed(arg)
                    }

                    ref pat => {
                        Error::custom("pattern not supported")
                            .with_span(pat)
                            .pipe(|e| errors.push(e));
                        FnArg::Typed(arg)
                    }
                }
            })
            .collect();

        let intent = match sig.asyncness {
            Some(token) => FunctionIntent::Awaited(token),
            None => FunctionIntent::Called,
        };

        let source = sig.ident.to_string().into();

        let this = FunctionThis::Self_;

        let output = match &sig.output {
            ReturnType::Default => None,
            output => V8Conv::from_output(output.clone())
                .and_recover(&mut errors)
                .pipe(Some),
        };

        let result = Self {
            intent,
            source,
            this,
            inputs,
            output,
        };

        let errors = errors.into_one();

        (result, errors).into()
    }
}

impl FunctionIntent {
    pub fn only(self, sig: &Signature) -> Caveat<Self> {
        let mut errors = Error::accumulator();

        let color = Self::some(sig).and_recover(&mut errors);

        match self {
            Self::Called | Self::Constructed => {
                if let Self::Awaited(span) = color {
                    Error::custom("fn cannot be `async` here")
                        .with_span(&span)
                        .pipe(|e| errors.push(e));
                }
            }
            Self::Awaited(_) => {
                if !matches!(color, Self::Awaited(_)) {
                    Error::custom("fn is required to be `async` here")
                        .with_span(&sig.fn_token)
                        .pipe(|e| errors.push(e));
                }
            }
        }

        (self, errors.into_one()).into()
    }

    pub fn some(sig: &Signature) -> Caveat<Self> {
        let color = match &sig.asyncness {
            None => Self::Called,
            Some(token) => Self::Awaited(*token),
        };
        (color, Self::supported(sig)).into()
    }

    pub fn supported(sig: &Signature) -> Option<Error> {
        let mut errors = Error::accumulator();

        macro_rules! deny {
            ($attr:ident, $msg:literal) => {
                if sig.$attr.is_some() {
                    Error::custom($msg)
                        .with_span(&sig.$attr)
                        .pipe(|e| errors.push(e));
                }
            };
        }

        deny!(constness, "fn cannot be `const` here");
        deny!(unsafety, "fn cannot be `unsafe` here");
        deny!(abi, "fn cannot be `extern` here");
        deny!(variadic, "fn cannot be variadic here");

        errors.into_one()
    }
}

impl ToTokens for FunctionIntent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let output = match self {
            Self::Called | Self::Constructed => quote! {},
            Self::Awaited(token) => quote! { #token },
        };
        tokens.extend(output);
    }
}

impl ToTokens for FunctionInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { ident, ty, .. } = self;
        tokens.extend(quote! { #ident: #ty });
    }
}
