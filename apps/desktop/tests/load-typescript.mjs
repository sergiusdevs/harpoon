import { readFile } from "node:fs/promises";
import ts from "typescript";

/**
 * Carga módulos de dominio usando TypeScript ya instalado. Los imports
 * relativos (`./paths`) se cargan igual, de forma recursiva; los de paquetes
 * se resuelven desde node_modules.
 */
export async function loadTypescript(relativePath, base = import.meta.url) {
  return import(await moduleUrl(new URL(relativePath, base)));
}

async function moduleUrl(url) {
  const file = url.pathname.endsWith(".ts") ? url : new URL(`${url.href}.ts`);
  const source = await readFile(file, "utf8");
  const { outputText: compiled } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  });
  // Solo las sentencias import/export reales, no textos que contengan `from "…"`.
  const statement = /^((?:import|export)\b[^;\n]*?\bfrom\s*)(["'])([^"']+)\2/gm;
  let outputText = compiled;
  for (const [match, head, , specifier] of [...compiled.matchAll(statement)]) {
    const target = specifier.startsWith(".") ? await moduleUrl(new URL(specifier, file)) : import.meta.resolve(specifier);
    outputText = outputText.replace(match, `${head}${JSON.stringify(target)}`);
  }
  return `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`;
}
