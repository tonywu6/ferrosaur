use std::time::Duration;

use anyhow::Result;
use deno_runtime::deno_core::v8;
use serde_json::json;

mod fixture;

use self::fixture::{deno::deno, Global};

#[tokio::test]
async fn test_functions() -> Result<()> {
    let (mut worker, main) = deno().await?;

    let rt = &mut worker.js_runtime;

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
    let (mut worker, main) = deno().await?;

    let rt = &mut worker.js_runtime;

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
    let (mut worker, main) = deno().await?;

    let rt = &mut worker.js_runtime;

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
    let (mut worker, _) = deno().await?;

    let rt = &mut worker.js_runtime;

    let global = Global::new(rt);

    let values = {
        let scope = &mut rt.handle_scope();
        let arg0 = v8::undefined(scope).cast::<v8::Value>();
        let arg1 = v8::Number::new(scope, 1.0).cast();
        let arg2 = v8::Boolean::new(scope, true).cast();
        [arg0, arg1, arg2].map(|a| v8::Global::new(scope, a))
    };

    // TODO: capture stdout

    Ok(())
}
