import { build, relpath } from "../_runtime/src/lib.ts";

await build({
  entryPoints: [relpath("src/lib.ts", import.meta)],
  outdir: "dist",
  mainFields: ["module", "main"],
  external: ["npm:typescript", "npm:@typescript/vfs"],
  bundle: true,
});

await build({
  entryPoints: [
    relpath("src/deps/typescript.ts", import.meta),
    relpath("src/deps/typescript-vfs.ts", import.meta),
  ],
  outdir: "dist",
  external: ["fs", "path", "os", "inspector"],
  mainFields: ["module", "main"],
  bundle: true,
  minify: false,
  treeShaking: true,
});
