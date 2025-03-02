import { createRequire } from "node:module";

const url = new URL("file:///globals.js");

const require = createRequire(url);
const __filename = url.pathname;
const __dirname = new URL(".", url).pathname;

Object.entries({
  require,
  __filename,
  __dirname,
}).forEach(([key, value]) =>
  Object.defineProperty(globalThis, key, {
    get: () => value,
    configurable: false,
    enumerable: true,
  }),
);
