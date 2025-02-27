export const sleep = (v, ms) =>
  new Promise((resolve) => setTimeout(() => resolve(v), ms));

export const useNavigate = () => (path) => console.log("navigating to", path);

export class Rectangle {
  constructor(width, height) {
    this.width = width;
    this.height = height;
  }

  area() {
    return this.width * this.height;
  }

  [Symbol.toPrimitive](hint) {
    if (hint === "number") {
      return this.area();
    }
    return `rect ${this.width}x${this.height}`;
  }

  maybeSquare() {
    if (this.width === this.height) {
      return this;
    } else {
      return null;
    }
  }
}

export class ThisConsideredHarmful {
  whoami() {
    return this;
  }
}
