// assuming cwd is CARGO_MANIFEST_DIR
export const cargoManifestDir = () => new URL(".", import.meta.url).pathname;

// import { __version__ } "./pkg.js"
export const pkgVersion = () =>
  import((void 0, "cargo:tests/js/pkg")).then((m) => m.__version__);
