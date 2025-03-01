declare module "ext:*" {}

declare module "ext:deno_url/00_url.js" {
  export const URL: typeof globalThis.URL;
}

declare module "ext:deno_console/01_console.js" {
  export class Console {
    constructor(fn: (msg: string, level: number) => void);
  }

  export function setNoColorFns(stdout: () => boolean, stderr: () => boolean): void;
}

declare module "ext:deno_web/02_timers.js" {
  export const setTimeout: typeof globalThis.setTimeout;
}
