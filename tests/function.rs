use std::time::Duration;

use anyhow::Result;
use deno_core::v8;
use serde_json::json;

mod fixture;

use self::fixture::deno::deno;

#[tokio::test]
async fn test_functions() -> Result<()> {
    let (mut rt, main) = deno().await?;

    let rt = &mut rt;

    let rectangle = main.rectangle(3.0, 4.0, rt)?;

    assert_eq!(rectangle.area(rt)?, 12.0);

    assert_eq!(rectangle.value(rt)?, json!("rect 3x4"));

    assert!(rectangle.square(rt)?.0.is_none());

    rectangle.set_width(4.0, rt)?;

    assert!(rectangle.square(rt)?.0.is_some());

    Ok(())
}

#[tokio::test]
async fn test_this() -> Result<()> {
    let (mut rt, main) = deno().await?;

    let rt = &mut rt;

    let checker = main.this_checker(rt)?;

    {
        let this = checker.get_this(rt)?;
        let scope = &mut rt.handle_scope();
        let this = v8::Local::new(scope, this);
        let checker = v8::Local::new(scope, checker.as_ref());
        assert!(this.same_value(checker));
    }

    {
        let this = checker.get_undefined(rt)?;
        let scope = &mut rt.handle_scope();
        let this = v8::Local::new(scope, this);
        assert!(this.is_undefined());
    }

    Ok(())
}

#[tokio::test]
async fn test_promise() -> Result<()> {
    let (mut rt, main) = deno().await?;

    let rt = &mut rt;

    let resolved = tokio::time::timeout(
        Duration::from_millis(60),
        main.sleep(true, Duration::from_millis(50).as_millis() as usize, rt),
    )
    .await??;

    assert!(resolved);

    Ok(())
}

#[tokio::test]
async fn test_variadic_fn() -> Result<()> {
    deno().await?;

    // TODO: capture stdout

    Ok(())
}
