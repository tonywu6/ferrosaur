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

  get [Symbol.toStringTag]() {
    return `Calculator: ${this.value}`;
  }
}

export const calc = new Calculator(0);

function* numbers() {
  yield "lorem";
  yield "ipsum";
  return "dolor";
}

export const gen = numbers();

export class Fibonacci {
  *[Symbol.iterator]() {
    let current = 0;
    let next = 1;

    for (;;) {
      [current, next] = [next, current + next];
      yield current;
    }
  }
}
