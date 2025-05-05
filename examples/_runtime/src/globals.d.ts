declare module "ext:*" {}

declare module "ext:core/mod.js" {
  export const core: {
    print: (...args: unknown[]) => void;
    ops: {
      op_example_cwd: () => string;
      op_example_read_file: (path: string) => Uint8Array;
    };
  };
}

declare module "ext:deno_url/00_url.js" {
  export const URL: typeof globalThis.URL;
}

declare module "ext:deno_console/01_console.js" {
  export interface Console {
    new (fn: (msg: string, level: number) => void): typeof globalThis.console;
  }
  export const Console: Console;
  export function setNoColorFns(stdout: () => boolean, stderr: () => boolean): void;
}

declare module "ext:deno_web/02_timers.js" {
  export const setTimeout: typeof globalThis.setTimeout;
}

declare module "ext:deno_web/05_base64.js" {
  export const atob: typeof globalThis.atob;
  export const btoa: typeof globalThis.btoa;
}

declare module "ext:deno_web/08_text_encoding.js" {
  export const TextDecoder: typeof globalThis.TextDecoder;
}
