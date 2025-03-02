import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const __filename = import.meta.filename;
const __dirname = import.meta.dirname;

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
