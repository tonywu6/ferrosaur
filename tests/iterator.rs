use anyhow::Result;

mod fixture;

use self::fixture::{deno::deno, Iter};

#[tokio::test]
async fn test_iterator() -> Result<()> {
    let (mut worker, _) = deno().await?;

    let rt = &mut worker.js_runtime;

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
