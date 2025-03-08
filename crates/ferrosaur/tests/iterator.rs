use std::collections::HashMap;

use anyhow::Result;
use compile::traits::Entries;

mod compile;
mod util;

use crate::{
    compile::modules::{I18n, Iter},
    util::deno,
};

#[tokio::test]
async fn test_iterator() -> Result<()> {
    let rt = &mut deno()?;

    let iter = Iter::main_module_init(rt).await?;

    let numbers = iter
        .fibonacci(20, rt)?
        .into_iter(rt)
        .collect::<Result<Vec<_>>>()?;

    assert_eq!(
        numbers,
        vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584, 4181]
    );

    Ok(())
}

#[tokio::test]
async fn test_iterator_trait() -> Result<()> {
    let rt = &mut deno()?;

    let i18n = I18n::main_module_init(rt).await?;

    let messages = i18n
        .messages(rt)
        .await?
        .entries(rt)?
        .into_iter(rt)
        .collect::<Result<HashMap<_, _>>>()?;

    assert_eq!(messages.len(), 3);

    Ok(())
}
