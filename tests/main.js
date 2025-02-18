export class Calculator {
  value = 0;

  constructor(value) {
    this.value = value;
  }

  add(value) {
    return new Calculator(this.value + value);
  }

  sub(value) {
    return new Calculator(this.value - value);
  }

  mul(value) {
    return new Calculator(this.value * value);
  }

  div(value) {
    return new Calculator(this.value / value);
  }
}

export const calc = new Calculator(0);
