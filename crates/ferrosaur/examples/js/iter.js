/**
 * @param {number} iter
 */
export function* fibonacci(iter) {
  let a = 0;
  let b = 1;
  for (let i = 0; i < iter; i++) {
    yield a;
    [a, b] = [b, a + b];
  }
}
