// @ts-check

import { build, relpath } from "../_runtime/src/lib.js";

await build({
  entryPoints: [relpath("src/typescript.ts", import.meta)],
  outdir: "dist",
  external: ["node:module", "node:process", "fs", "path", "os", "inspector"],
  mainFields: ["module", "main"],
  bundle: true,
  minify: false,
  treeShaking: true,
});

await build({
  entryPoints: [relpath("src/lib.ts", import.meta)],
  outdir: "dist",
});
