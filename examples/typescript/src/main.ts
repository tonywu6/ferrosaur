import "./globals.ts";

import * as ts from "typescript";

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
    printDiagnostics: () => {
      const diagnostics = program.getSemanticDiagnostics();
      return ts.formatDiagnosticsWithColorAndContext(diagnostics, diagnosticsHost);
    },
  };
}

function createCompilerHost() {
  const host = ts.createCompilerHost({});

  const { readFile } = host;

  const runtimeDir = new URL(".", import.meta.url).pathname;

  host.readFile = (fileName) => {
    if (fileName.startsWith(runtimeDir)) {
      const suffix = fileName.slice(runtimeDir.length);
      const content = globalThis.__TYPESCRIPT_LIB__?.[suffix];
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
    // deno-lint-ignore no-var
    var __TYPESCRIPT_LIB__: Record<string, string> | undefined;
  }
}

const diagnosticsHost: ts.FormatDiagnosticsHost = {
  getCurrentDirectory: () => Deno.cwd(),
  getCanonicalFileName: (name) => name,
  getNewLine: () => "\n",
};
