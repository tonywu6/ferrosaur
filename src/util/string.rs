use darling::{Error, FromMeta, Result};
use syn::{Lit, Meta};
use tap::Pipe;

#[derive(Debug, Default, Clone)]
pub struct StringLike<T>(pub T);

impl<T: FromMeta> FromMeta for StringLike<T> {
    fn from_meta(item: &Meta) -> Result<Self> {
        let key = if let Meta::Path(path) = item {
            if path.segments.len() == 1 && path.segments[0].arguments.is_none() {
                Some(path.segments[0].ident.to_string())
            } else {
                None
            }
        } else {
            None
        };
        if let Some(key) = key {
            Self::from_string(&key)
        } else {
            "expected an identifier, a string, or a number"
                .pipe(Error::custom)
                .with_span(item)
                .pipe(Err)
        }
    }

    fn from_value(value: &Lit) -> Result<Self> {
        match value {
            Lit::Str(s) => Self::from_string(&s.value()),
            Lit::Char(ch) => Self::from_char(ch.value()),
            Lit::Int(n) => Self::from_string(n.base10_digits()),
            Lit::Float(f) => Self::from_string(f.base10_digits()),
            _ => Err(Error::unexpected_lit_type(value)),
        }
        .map_err(|e| e.with_span(value))
    }

    fn from_string(value: &str) -> Result<Self> {
        Ok(Self(T::from_string(value)?))
    }

    fn from_char(value: char) -> Result<Self> {
        Ok(Self(T::from_char(value)?))
    }
}
