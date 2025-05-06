// This example showcases ES module loading and method chaining.
//
// To run this example, run:
//
//     cargo run --package example-calculator
//
// ---

// Use [`#[js(module)]`][js-module] to embed an [ECMAScript module]
// into your Rust program:

use ferrosaur::js;

#[js(module("./main.js", fast))]
struct Main;

// ---

// [`main.js`](/examples/calculator/src/main.js) exports a `Calculator` class:

#[js(interface)]
impl Main {
    #[js(new)]
    fn calculator(&self, value: serde<f64>) -> Calculator {}
    // export class Calculator ...
}

//

#[js(value)]
#[derive(Debug)]
struct Calculator;

#[js(interface)]
impl Calculator {
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

    #[js(prop)]
    fn value(&self) -> serde<f64> {}
}

// ---

// Here's the main function:

#[tokio::main]
async fn main() -> Result<()> {
    let rt = &mut JsRuntime::new(RuntimeOptions::default());

    // Initialize the module:
    let main = Main::main_module_init(rt).await?;

    let calc = main
        .calculator(1.0, rt)?
        .add(2.0, rt)?
        .sub(3.0, rt)?
        .mul(4.0, rt)?
        .div(5.0, rt)?;
    // This is https://oeis.org/A261038

    println!("RPN: {}", calc.print(rt)?);
    // https://en.wikipedia.org/wiki/Reverse_Polish_notation

    assert_eq!(calc.value(rt)?, 0.0);

    Ok(())
}

// ---
//
// <details>
//   <summary>Additional setup code for this example</summary>

use anyhow::Result;
use deno_core::{JsRuntime, RuntimeOptions};

// </details>

// [ECMAScript module]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules
// [js-module]:         /docs/src/reference/module.md
