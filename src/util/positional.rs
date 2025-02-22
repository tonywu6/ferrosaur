use darling::{ast::NestedMeta, Error, FromMeta, Result};

#[derive(Debug, Default, Clone, Copy)]
pub struct Positional<P, T> {
    pub head: P,
    pub rest: T,
}

pub trait FromPositional: Sized {
    fn fallback() -> Result<Self>;
}

impl<P, T> FromMeta for Positional<P, T>
where
    P: FromMeta + FromPositional,
    T: FromMeta,
{
    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        match items.len() {
            0 => Ok(Self {
                head: P::fallback()?,
                rest: T::from_list(&[])?,
            }),
            _ => {
                match T::from_list(items)
                    .and_then(|rest| P::fallback().map(|head| Self { head, rest }))
                {
                    Ok(this) => Ok(this),
                    Err(e1) => match T::from_list(&items[1..]).and_then(|rest| {
                        P::from_nested_meta(&items[0]).map(|head| Self { head, rest })
                    }) {
                        Ok(this) => Ok(this),
                        Err(e2) => Err(Error::multiple(vec![e2, e1])),
                    },
                }
            }
        }
    }
}
