// @ts-check

import { build, relpath } from "../_runtime/src/lib.js";

await build({
  entryPoints: [relpath("src/main.ts", import.meta)],
  outdir: "dist",
  bundle: true,
  external: ["npm:typescript"],
  plugins: [npmTypeScript()],
});

/**
 * @returns {import("esbuild").Plugin}
 */
function npmTypeScript() {
  return {
    name: "npm:typescript",
    setup: (build) => {
      build.onLoad(
        { filter: /ts-blank-space\/out\/index.js/, namespace: "file" },
        async ({ path }) => {
          let contents = await Deno.readTextFile(path);
          contents = contents.replace(
            `import tslib from "typescript";`,
            `import tslib from "npm:typescript";`,
          );
          return { contents, loader: "js" };
        },
      );
    },
  };
}
