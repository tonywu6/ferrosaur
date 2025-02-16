use darling::{
    ast::NestedMeta, error::Accumulator, util::path_to_string, Error, FromGenerics, FromMeta,
    Result,
};
use heck::ToSnakeCase;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Paren, Attribute, Generics, Ident, Meta, Path,
    PathSegment, Token, VisRestricted, Visibility,
};
use tap::{Conv, Pipe, Tap};

pub trait TokenStreamResult {
    fn or_error(self) -> TokenStream;
}

impl TokenStreamResult for Result<TokenStream> {
    fn or_error(self) -> TokenStream {
        self.unwrap_or_else(Error::write_errors)
    }
}

pub trait ReturnWithErrors<T> {
    fn or_return_with(self, errors: Accumulator) -> Result<(T, Accumulator)>;
}

impl<T> ReturnWithErrors<T> for Result<T> {
    fn or_return_with(self, errors: Accumulator) -> Result<(T, Accumulator)> {
        match self {
            Ok(value) => Ok((value, errors)),
            Err(error) => errors
                .tap_mut(|errors| errors.push(error))
                .finish()
                .map(|_| unreachable!()),
        }
    }
}

pub trait FromMetaList: Sized {
    fn from_meta_list(tokens: TokenStream) -> Result<Self>;
}

impl<T: FromMeta> FromMetaList for T {
    fn from_meta_list(tokens: TokenStream) -> Result<Self> {
        NestedMeta::parse_meta_list(tokens)?.pipe_as_ref(Self::from_list)
    }
}

pub trait FromMetaEnum: FromMeta {
    fn test(name: &str) -> bool;

    fn from_unit(name: &str) -> Result<Self>;

    fn from_meta2(meta: &Meta) -> Result<Self> {
        match meta {
            Meta::Path(path) => Self::from_unit(&path_to_string(path)),
            Meta::List(..) => Self::from_list(&[NestedMeta::Meta(meta.clone())]),
            meta => Self::from_meta(meta),
        }
    }

    fn filter_attrs(
        attrs: Vec<Attribute>,
        errors: &mut Accumulator,
    ) -> (Vec<Self>, Vec<Attribute>) {
        let mut items = Vec::<Self>::new();
        let attrs = attrs
            .into_iter()
            .filter_map(|attr| {
                if Self::test(&path_to_string(attr.meta.path())) {
                    if let Some(item) = errors.handle(Self::from_meta2(&attr.meta)) {
                        items.push(item);
                        None
                    } else {
                        Some(attr)
                    }
                } else {
                    Some(attr)
                }
            })
            .collect::<Vec<_>>();
        (items, attrs)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoGenerics;

impl FromGenerics for NoGenerics {
    fn from_generics(generics: &Generics) -> Result<Self> {
        let mut errors = Error::accumulator();

        if !generics.params.is_empty() {
            Error::custom("must not have generics")
                .with_span(&generics.params)
                .pipe(|e| errors.push(e))
        }

        if generics.where_clause.is_some() {
            Error::custom("must not have a where clause")
                .with_span(&generics.where_clause)
                .pipe(|e| errors.push(e));
        }

        errors.finish_with(Self)
    }
}

#[derive(Debug, Clone, FromMeta)]
pub struct DenoCorePath(Path);

impl Default for DenoCorePath {
    fn default() -> Self {
        Ident::new("deno_core", Span::call_site())
            .conv::<PathSegment>()
            .conv::<Path>()
            .pipe(Self)
    }
}

impl ToTokens for DenoCorePath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens)
    }
}

pub fn inner_mod_name<T: ToTokens>(prefix: &str, item: T) -> Ident {
    fn collect_ident(stream: TokenStream, collector: &mut Vec<String>) {
        for token in stream {
            match token {
                TokenTree::Ident(ident) => collector.push(ident.to_string().to_snake_case()),
                TokenTree::Group(group) => collect_ident(group.stream(), collector),
                _ => {}
            }
        }
    }
    let name = {
        let mut tokens = vec![];
        collect_ident(item.to_token_stream(), &mut tokens);
        tokens
    }
    .join("_");
    format!("__{prefix}_{}_bindgen", name)
        .to_lowercase()
        .pipe_as_ref(|name| Ident::new(name, item.span()))
}

#[allow(unused)]
pub fn pub_in_super(vis: Visibility) -> Visibility {
    match vis {
        Visibility::Public(..) => vis,
        Visibility::Restricted(vis) => {
            let span = vis.span();
            if vis.path.segments.first().map(|s| &s.ident) == Some(&Token![super](span).into()) {
                let VisRestricted {
                    pub_token,
                    paren_token,
                    in_token,
                    path: suffix,
                } = vis;
                let prefix = Token![super](span).conv::<Ident>().conv::<PathSegment>();
                let path = Punctuated::<PathSegment, Token![::]>::new()
                    .tap_mut(|p| p.push(prefix))
                    .tap_mut(|p| p.extend(suffix.segments))
                    .pipe(|segments| Path {
                        segments,
                        leading_colon: None,
                    })
                    .into();
                VisRestricted {
                    pub_token,
                    paren_token,
                    in_token: in_token.unwrap_or_default().pipe(Some),
                    path,
                }
            } else {
                vis
            }
            .pipe(Visibility::Restricted)
        }
        Visibility::Inherited => VisRestricted {
            pub_token: Token![pub](vis.span()),
            in_token: None,
            paren_token: Paren(vis.span()),
            path: Token![super](vis.span()).conv::<Path>().pipe(Box::new),
        }
        .pipe(Visibility::Restricted),
    }
}

pub fn use_prelude() -> TokenStream {
    quote! {
        #[allow(unused)]
        use ::core::{
            convert::{AsRef, From, Into},
            default::Default,
            option::Option::{self, None, Some},
            result::Result::{Err, Ok},
        };
    }
}
