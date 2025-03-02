// @ts-check

import * as esbuild from "esbuild";

/**
 * @param {string[]} entryPoints
 */
export async function buildExample(entryPoints) {
  const result = await esbuild.build({
    entryPoints,
    outdir: "dist",
    metafile: true,

    format: "esm",
    platform: "neutral",
    target: ["esnext", "deno2"],
    external: ["node:module", "node:process", "fs", "path", "os", "inspector"],
    mainFields: ["module", "main"],

    bundle: true,
    minify: true,
    treeShaking: true,
  });

  Object.keys(result.metafile.inputs).forEach((path) => {
    console.log(`cargo::rerun-if-changed=${path}`);
  });
}

/**
 * @param {string} path
 * @param {ImportMeta} meta
 * @returns
 */
export const relpath = (path, meta) => new URL(path, meta.url).pathname;
