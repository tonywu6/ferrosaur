use darling::Error;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    spanned::Spanned, token::Paren, Expr, ExprPath, FnArg, Generics, Ident, Pat, PatIdent,
    PatRange, PatType, Path, RangeLimits, Receiver, ReturnType, Signature, Token, Type, TypePath,
    TypeTuple,
};
use tap::{Pipe, Tap};

use super::{
    property::PropertyKey, unwrap_v8_local, v8::V8Conv, Caveat, MergeErrors, RecoverableErrors,
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

#[derive(Debug, Clone)]
pub struct BindFunction {
    pub source: FunctionSource,
    pub this: FunctionThis,
    pub ctor: bool,
    pub length: FunctionLength,
}

#[derive(Debug, Clone)]
pub enum FunctionSource {
    Prop(PropertyKey),
    This,
}

#[derive(Debug, Clone, Copy)]
pub enum FunctionLength {
    Fixed(usize),
    Variadic,
}

#[derive(Debug, Clone, Copy)]
pub enum FunctionThis {
    Self_,
    Undefined { this: Span, undefined: Span },
    Unbound,
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
            match self.intent {
                FunctionIntent::Called | FunctionIntent::Awaited(_) => match &self.source {
                    FunctionSource::Prop(prop) => format!("failed to call function {prop:?}"),
                    FunctionSource::This => "failed to call function".into(),
                },
                FunctionIntent::Constructed => match &self.source {
                    FunctionSource::Prop(prop) => format!("failed to construct {prop:?}"),
                    FunctionSource::This => "failed to construct".into(),
                },
            }
        };

        let (casts, length) = {
            let variadic = self
                .inputs
                .iter()
                .any(|FunctionInput { spread, .. }| *spread);

            let length = if variadic {
                FunctionLength::Variadic
            } else {
                FunctionLength::Fixed(self.inputs.len())
            };

            let casts = if self.inputs.is_empty() {
                if variadic {
                    quote! { vec![] }
                } else {
                    quote! { [] }
                }
            } else if self
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

                quote! {{
                    let __scope = &mut _rt.handle_scope();
                    let mut __args = Vec::new();
                    #(#casts)*
                    __args
                }}
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

                quote! {{
                    let __scope = &mut _rt.handle_scope();
                    #(#casts)*
                    [#(#names),*]
                }}
            };

            (casts, length)
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

        let mut this = FunctionThis::Self_;

        sig.inputs = std::mem::take(&mut sig.inputs)
            .into_iter()
            .filter_map(|arg| {
                let FnArg::Typed(arg) = arg else {
                    return Some(arg);
                };

                let ty = V8Conv::from_type((*arg.ty).clone()).and_recover(&mut errors);

                match *arg.pat {
                    Pat::Ident(PatIdent {
                        ref ident,
                        by_ref: None,
                        mutability: None,
                        subpat: None,
                        ..
                    }) => {
                        if ident == "this" {
                            if let Some(undefined) = is_undefined(&arg) {
                                this = undefined;
                                return None;
                            } else {
                                this = FunctionThis::Unbound
                            }
                        }

                        let spread = false;
                        let ident = ident.clone();
                        let arg = arg.tap_mut(|arg| arg.ty = ty.to_type().into());

                        inputs.push(FunctionInput { ident, ty, spread });
                        Some(FnArg::Typed(arg))
                    }

                    Pat::Range(PatRange {
                        start: None,
                        end: Some(ref end),
                        limits: RangeLimits::HalfOpen(..),
                        ..
                    }) => match &**end {
                        Expr::Path(path) if is_unit_path(path) => {
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
                            Some(FnArg::Typed(arg))
                        }
                        expr => {
                            "spread argument should be written as `..name`\nfound extra patterns"
                                .pipe(Error::custom)
                                .with_span(expr)
                                .pipe(|e| errors.push(e));
                            Some(FnArg::Typed(arg))
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
                        Some(FnArg::Typed(arg))
                    }

                    ref pat => {
                        Error::custom("expected an identifier or spread argument")
                            .with_span(pat)
                            .pipe(|e| errors.push(e));
                        Some(FnArg::Typed(arg))
                    }
                }
            })
            .collect();

        let intent = match sig.asyncness {
            Some(token) => FunctionIntent::Awaited(token),
            None => FunctionIntent::Called,
        };

        let source = sig.ident.to_string().into();

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

impl ToTokens for BindFunction {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let func = V8Conv::new_v8("Function");

        let get_func = match &self.source {
            FunctionSource::Prop(prop) => {
                let getter = func.to_getter();
                quote! {{
                    #getter
                    let prop = #prop;
                    getter(scope, object, prop)
                        .context("failed to get function object")?
                }}
            }
            FunctionSource::This => {
                quote! {{
                    object.try_cast::<v8::Function>()
                        .context("failed to cast self as function")?
                }}
            }
        };

        let object_ty = match &self.source {
            FunctionSource::Prop(_) => quote! { v8::Object },
            FunctionSource::This => quote! { v8::Function },
        };

        let get_bind = {
            let getter = func.to_getter();
            let prop = PropertyKey::from("bind");
            quote! {{
                #getter
                let prop = #prop;
                getter(scope, object, prop)
                    .context("failed to get Function.property.bind from function")?
            }}
        };

        let args_ty = match self.length {
            FunctionLength::Fixed(len) => quote! {
                [v8::Global<v8::Value>; #len]
            },
            FunctionLength::Variadic => quote! {
                Vec<v8::Global<v8::Value>>
            },
        };

        let this = match self.this {
            FunctionThis::Self_ => Some(quote! {{
                let object = TryInto::try_into(object)
                    .context("failed to cast `self` as a v8::Object")?;
                object.cast::<v8::Value>()
            }}),
            FunctionThis::Undefined { this, undefined } => {
                let this = Ident::new("this", this);
                let undefined = Ident::new("undefined", undefined);
                Some(quote! {{
                    let #this = v8::#undefined(scope);
                    #this.cast::<v8::Value>()
                }})
            }
            FunctionThis::Unbound => None,
        };

        let args = if let Some(this) = this {
            match self.length {
                FunctionLength::Fixed(len) => {
                    let locals = (0..len).map(|idx| {
                        let offset = idx + 1;
                        quote! {
                            array[#offset] = v8::Local::new(scope, &args[#idx]);
                        }
                    });
                    let total_len = len + 1;
                    quote! {{
                        let undefined = v8::undefined(scope).cast::<v8::Value>();
                        let mut array = [undefined; #total_len];
                        let this = #this;
                        array[0] = this;
                        #(#locals)*
                        array
                    }}
                }
                FunctionLength::Variadic => quote! {{
                    let this = #this;
                    ::core::iter::once(this)
                        .chain(args.iter().map(|arg| v8::Local::new(scope, arg)))
                        .collect::<Vec<_>>()
                }},
            }
        } else {
            match self.length {
                FunctionLength::Fixed(_) => quote! {{
                    args.map(|arg| v8::Local::new(scope, arg))
                }},
                FunctionLength::Variadic => quote! {{
                    args.into_iter()
                        .map(|arg| v8::Local::new(scope, arg))
                        .collect::<Vec<_>>()
                }},
            }
        };

        let unwrap_callable = unwrap_v8_local("callable");

        let retval = if self.ctor {
            quote! {{
                callable.new_instance(scope, &[])
            }}
        } else {
            quote! {{
                let recv = v8::undefined(scope);
                callable.call(scope, recv.into(), &[])
            }}
        };

        let unwrap_retval = unwrap_v8_local("retval");

        let result = quote! {
            #[inline(always)]
            fn call<'a, T>(
                scope: &mut v8::HandleScope<'a>,
                object: v8::Local<'a, T>,
                #[allow(unused)]
                args: #args_ty,
            ) -> Result<v8::Global<v8::Value>>
            where
                v8::Local<'a, T>: TryInto<v8::Local<'a, #object_ty>,
                    Error: ::core::error::Error + Send + Sync + 'static>,
            {
                let func = #get_func;
                let bind = {
                    let object = v8::Local::new(scope, &func);
                    #get_bind
                };
                let bind = v8::Local::new(scope, bind);
                let func = v8::Local::new(scope, func);
                let func = Into::into(func);
                let args = #args;
                let scope = &mut v8::TryCatch::new(scope);
                let callable = bind.call(scope, func, &args);
                let callable = #unwrap_callable;
                let callable = callable.try_cast::<v8::Function>()?;
                let retval = #retval;
                let retval = #unwrap_retval.try_cast()?;
                Ok(v8::Global::new(scope, retval))
            }
        };

        tokens.extend(result);
    }
}

impl<T> From<T> for FunctionSource
where
    T: Into<PropertyKey>,
{
    fn from(value: T) -> Self {
        Self::Prop(value.into())
    }
}

#[inline(always)]
fn is_undefined(arg: &PatType) -> Option<FunctionThis> {
    let Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            ref segments,
        },
    }) = &*arg.ty
    else {
        return None;
    };
    if segments.len() != 1 || segments[0].ident != "undefined" || !segments[0].arguments.is_none() {
        return None;
    }
    let this = arg.pat.span();
    let undefined = arg.ty.span();
    Some(FunctionThis::Undefined { this, undefined })
}

#[inline(always)]
fn is_unit_path(path: &ExprPath) -> bool {
    path.path.segments.len() == 1 && path.path.leading_colon.is_none() && path.qself.is_none()
}
