/**
 * @param {number} n
 * @returns {number}
 */
export const slowFib = (n) =>
  n === 0 ? 0 : n === 1 ? 1 : slowFib(n - 1) + slowFib(n - 2);
