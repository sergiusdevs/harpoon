import { readFile } from "node:fs/promises";
import ts from "typescript";

/** Carga módulos de dominio sin dependencias, usando TypeScript ya instalado. */
export async function loadTypescript(relativePath) {
  const source = await readFile(new URL(relativePath, import.meta.url), "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  });
  return import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
}
