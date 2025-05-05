/**
 * @template T
 * @param {T} v
 * @param {number} ms
 * @returns {Promise<T>}
 */
export const sleep = (v, ms) =>
  new Promise((resolve) => setTimeout(() => resolve(v), ms));

/**
 * @returns {(path: string) => void}
 */
export const useNavigate = () => (path) => console.log("navigating to", path);

class Shape {
  /**
   * @returns {number}
   */
  area() {
    throw new Error("not implemented");
  }

  /**
   * @param {string} hint
   */
  [Symbol.toPrimitive](hint) {
    if (hint === "number") {
      return this.area();
    } else {
      return this[Symbol.toStringTag]();
    }
  }

  [Symbol.toStringTag]() {
    return "shape";
  }
}

export class Rectangle extends Shape {
  /**
   * @param {number} width
   * @param {number} height
   */
  constructor(width, height) {
    super();
    /** @type {number} */
    this.width = width;
    /** @type {number} */
    this.height = height;
  }

  /** @override */
  area() {
    return this.width * this.height;
  }

  /** @override */
  [Symbol.toStringTag]() {
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
