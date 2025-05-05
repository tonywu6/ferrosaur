import * as esbuild from "esbuild";

export const relpath = (path: string, meta: ImportMeta) =>
  new URL(path, meta.url).pathname;

export async function build(
  options: Omit<esbuild.BuildOptions, "format" | "platform" | "target" | "metafile">,
) {
  const {
    metafile: { inputs },
  } = await esbuild.build({
    ...options,
    format: "esm",
    platform: "neutral",
    target: ["esnext", "deno2"],
    metafile: true,
  });
  for (const path of Object.keys(inputs)) {
    console.log(`cargo::rerun-if-changed=${path}`);
  }
}

export function moduleSpecifier({
  external = [],
  compiled = [],
}: {
  external?: string[];
  compiled?: string[];
}): esbuild.Plugin {
  return {
    name: "module-specifier",
    setup: (build) => {
      build.onResolve(
        {
          filter: new RegExp(`^${external.join("|")}$`),
          namespace: "file",
        },
        ({ path }) => {
          if (!path.startsWith("npm:")) {
            path = `npm:${path}`;
          }
          return { path, external: true };
        },
      );
      build.onResolve(
        {
          filter: new RegExp(`^${compiled.join("|")}$`),
          namespace: "file",
        },
        async ({ path, ...args }) => {
          if (path.startsWith("npm:")) {
            path = path.slice(4);
          }
          return await build.resolve(path, args);
        },
      );
    },
  };
}
