use anyhow::Result;

mod compile;
mod util;

use crate::{compile::modules::Iter, util::deno};

#[tokio::test]
async fn test_iterator() -> Result<()> {
    let rt = &mut deno().await?;

    let iter = Iter::new(rt).await?;

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
