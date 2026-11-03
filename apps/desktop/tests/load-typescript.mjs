import { readFile } from "node:fs/promises";
import ts from "typescript";

/** Carga módulos de dominio sin dependencias, usando TypeScript ya instalado. */
export async function loadTypescript(relativePath) {
  const source = await readFile(new URL(relativePath, import.meta.url), "utf8");
  const { outputText: compiled } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  });
  const outputText = compiled.replace(/from (["'])([^."'][^"']*)\1/g, (_match, _quote, specifier) => `from ${JSON.stringify(import.meta.resolve(specifier))}`);
  return import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
}
