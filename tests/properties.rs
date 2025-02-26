use std::time::Duration;

use anyhow::Result;
use deno_bindgen3::js;
use deno_core::v8;
use serde_json::json;

mod compile;
mod util;

use crate::{
    compile::{Global, I18n, Main},
    util::deno,
};

#[tokio::test]
async fn test_functions() -> Result<()> {
    let rt = &mut deno().await?;

    let main = Main::new(rt).await?;

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
    let rt = &mut deno().await?;

    let main = Main::new(rt).await?;

    let checker = main.this_checker(rt)?;

    let this_1 = checker.get_this(rt)?;
    let this_2 = checker.get_unbound(main.as_ref().clone(), rt)?;
    let this_3 = checker.get_undefined(rt)?;

    {
        let scope = &mut rt.handle_scope();
        let checker = v8::Local::new(scope, checker.as_ref());
        let main = v8::Local::new(scope, main.as_ref()).into();
        let this_1 = v8::Local::new(scope, this_1);
        let this_2 = v8::Local::new(scope, this_2);
        let this_3 = v8::Local::new(scope, this_3);
        assert!(this_1.same_value(checker));
        assert!(this_2.same_value(main));
        assert!(this_3.is_undefined());
    }

    Ok(())
}

#[tokio::test]
async fn test_promise() -> Result<()> {
    let rt = &mut deno().await?;

    let main = Main::new(rt).await?;

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
    let rt = &mut deno().await?;

    let global = Global::new(rt);

    global.console(rt)?.log(
        &[
            global.number(0.0, rt)?,
            global.boolean(true, rt)?,
            global.string("2", rt)?,
            global.date(3.0, rt)?,
        ],
        rt,
    )?;

    let stdout = global.cargo_test_stdout(rt)?;

    insta::assert_snapshot!(stdout);

    Ok(())
}

#[js(properties)]
impl Global {
    #[js(func(name(Boolean)))]
    fn boolean(&self, v: serde<bool>) -> v8::Global<v8::Value> {}

    #[js(func(name(Number)))]
    fn number(&self, v: serde<f64>) -> v8::Global<v8::Value> {}

    #[js(func(name(String)))]
    fn string(&self, v: serde<&str>) -> v8::Global<v8::Value> {}

    #[js(new(class(Date)))]
    fn date(&self, v: serde<f64>) -> v8::Global<v8::Value> {}
}

#[tokio::test]
async fn test_indexing_get() -> Result<()> {
    let rt = &mut deno().await?;

    let i18n = I18n::new(rt).await?;

    assert_eq!("https://zh.wikipedia.org/wiki/千字文", i18n.zh_cn(rt)?);

    assert_eq!(
        i18n.i18n(
            "Franz jagt im komplett verwahrlosten Taxi quer durch Bayern",
            rt,
        )?,
        "https://de.wikipedia.org/wiki/Pangramm",
    );

    assert!(i18n.i18n("", rt).is_err());

    Ok(())
}

#[tokio::test]
async fn test_indexing_set() -> Result<()> {
    let rt = &mut deno().await?;

    let global = Global::new(rt);

    let i18n = {
        let i18n = I18n::new(rt).await?;
        let scope = &mut rt.handle_scope();
        let i18n = v8::Local::new(scope, i18n.as_ref());
        v8::Global::new(scope, i18n.cast::<v8::Value>())
    };

    global.declare("foo", i18n, rt)?;

    {
        let typeof_ = rt.execute_script("", "typeof foo")?;
        let scope = &mut rt.handle_scope();
        let typeof_ = v8::Local::new(scope, typeof_);
        assert_eq!(typeof_.to_rust_string_lossy(scope), "object")
    };

    Ok(())
}
