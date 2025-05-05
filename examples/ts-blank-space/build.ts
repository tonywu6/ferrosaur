import { build, relpath, moduleSpecifier } from "../_runtime/src/lib.ts";

await build({
  entryPoints: [relpath("src/main.ts", import.meta)],
  outdir: "dist",
  bundle: true,
  mainFields: ["module", "main"],
  plugins: [
    moduleSpecifier({
      external: ["typescript"],
    }),
  ],
});
