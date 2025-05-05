export class Calculator {
  /** @type {number} */
  value;

  /** @type {(string|number)[]} */
  history;

  /** @param {number} value */
  constructor(value = 0) {
    this.value = value;
    this.history = [value];
  }

  /** @param {number} value */
  add(value) {
    return Calculator.derive(this, "+", value);
  }

  /** @param {number} value */
  sub(value) {
    return Calculator.derive(this, "-", value);
  }

  /** @param {number} value */
  mul(value) {
    return Calculator.derive(this, "*", value);
  }

  /** @param {number} value */
  div(value) {
    return Calculator.derive(this, "/", value);
  }

  /** @param {unknown} hint */
  [Symbol.toPrimitive](hint) {
    switch (hint) {
      case "number":
        return this.value;
      default:
        return `${this.history.join(" ")} = ${this.value}`;
    }
  }

  /**
   * @param {Calculator} self
   * @param {'+' | '-' | '*' | '/'} op
   * @param {number} rhs
   */
  static derive(self, op, rhs) {
    /** @type {number} */
    let value;
    switch (op) {
      case "+":
        value = self.value + rhs;
        break;
      case "-":
        value = self.value - rhs;
        break;
      case "*":
        value = self.value * rhs;
        break;
      case "/":
        value = self.value / rhs;
        break;
      default:
        throw new Error("Invalid operator");
    }
    const next = new Calculator(value);
    next.history = [...self.history, rhs, op];
    return next;
  }
}
