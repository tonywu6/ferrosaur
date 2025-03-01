const enUS =
  "https://en.wikipedia.org/wiki/The_quick_brown_fox_jumps_over_the_lazy_dog";

const deDE = "https://de.wikipedia.org/wiki/Pangramm";

const zhCN = "https://zh.wikipedia.org/wiki/千字文";

export {
  enUS as "The quick brown fox jumps over the lazy dog",
  deDE as "Franz jagt im komplett verwahrlosten Taxi quer durch Bayern",
  zhCN as "天地玄黄，宇宙洪荒",
};

export async function messages() {
  const self = await import("./i18n.js");
  return new Map(Object.entries(self).filter(([, v]) => typeof v === "string"));
}
