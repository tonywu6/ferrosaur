// @ts-check

import * as esbuild from "esbuild";

/**
 * @param {Omit<
 *  esbuild.BuildOptions,
 *  'format' | 'platform'| 'target' | 'metafile'
 * >} options
 */
export async function build(options) {
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

/**
 * @param {string} path
 * @param {ImportMeta} meta
 */
export const relpath = (path, meta) => new URL(path, meta.url).pathname;
