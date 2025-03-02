use darling::{ast::NestedMeta, FromMeta, Result};

#[derive(Debug, Default, Clone, Copy)]
pub struct Positional<P, T> {
    pub head: P,
    pub rest: T,
}

impl<P, T> FromMeta for Positional<P, T>
where
    P: FromMeta,
    T: FromMeta,
{
    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        match items.len() {
            0 => Ok(Self {
                head: P::from_list(&[])?,
                rest: T::from_list(&[])?,
            }),
            _ => Ok(Self {
                head: P::from_nested_meta(&items[0])?,
                rest: T::from_list(&items[1..])?,
            }),
        }
    }
}
