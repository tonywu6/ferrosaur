import ts from "npm:typescript";

export function createProgram(...files: string[]) {
  const host = createCompilerHost();

  const program = ts.createProgram(
    files,
    {
      strict: true,
      noEmit: true,
      target: ts.ScriptTarget.ESNext,
      lib: ["ESNext"],
      module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.Bundler,
      moduleDetection: ts.ModuleDetectionKind.Force,
    },
    host,
  );

  return {
    printDiagnostics: (colored = true) => {
      const diagnostics = program.getSemanticDiagnostics();
      if (colored) {
        return ts.formatDiagnosticsWithColorAndContext(diagnostics, diagnosticsHost);
      } else {
        return ts.formatDiagnostics(diagnostics, diagnosticsHost);
      }
    },
  };
}

function createCompilerHost(): ts.CompilerHost {
  const host = ts.createCompilerHost({});

  const { readFile } = host;

  const runtimeDir = new URL(".", import.meta.url).pathname;

  host.getDefaultLibLocation = () => runtimeDir;

  host.readFile = (fileName) => {
    if (fileName.startsWith(runtimeDir)) {
      const suffix = fileName.slice(runtimeDir.length);
      const content = globalThis.TYPESCRIPT_LIB?.[suffix];
      if (typeof content === "string") {
        return content;
      }
    }
    return readFile(fileName);
  };

  return host;
}

declare global {
  namespace globalThis {
    var TYPESCRIPT_LIB: Record<string, string> | undefined;
    var CARGO_MANIFEST_DIR: string | undefined;
  }
}

const diagnosticsHost: ts.FormatDiagnosticsHost = {
  getCurrentDirectory: () => globalThis.CARGO_MANIFEST_DIR || Deno.cwd(),
  getCanonicalFileName: (name) => name,
  getNewLine: () => "\n",
};
