use anyhow::Result;
use deno_core::{JsRuntime, RuntimeOptions};
use ferrosaur::js;

#[js(module("./main.js", fast))]
struct Main;

#[js(interface)]
impl Main {
    #[js(new)]
    fn calculator(&self, value: serde<f64>) -> Calculator {}
}

#[js(value)]
#[derive(Debug)]
struct Calculator;

#[js(interface)]
impl Calculator {
    #[js(prop)]
    fn value(&self) -> serde<f64> {}

    #[js(func)]
    fn add(&self, value: serde<f64>) -> Self {}

    #[js(func)]
    fn sub(&self, value: serde<f64>) -> Self {}

    #[js(func)]
    fn mul(&self, value: serde<f64>) -> Self {}

    #[js(func)]
    fn div(&self, value: serde<f64>) -> Self {}

    #[js(func(Symbol(toPrimitive)))]
    fn print(&self) -> String {}
}

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut JsRuntime::new(RuntimeOptions::default());

    // https://oeis.org/A261038
    let calc = Main::main_module_init(rt)
        .await?
        .calculator(1.0, rt)?
        .add(2.0, rt)?
        .sub(3.0, rt)?
        .mul(4.0, rt)?
        .div(5.0, rt)?;

    // https://en.wikipedia.org/wiki/Reverse_Polish_notation
    println!("RPN: {}", calc.print(rt)?);

    assert_eq!(calc.value(rt)?, 0.0);

    Ok(())
}
