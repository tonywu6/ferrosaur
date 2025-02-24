declare module "ext:*" {}

declare module "ext:deno_web/02_timers.js" {
  export const setTimeout: typeof globalThis.setTimeout;
}
