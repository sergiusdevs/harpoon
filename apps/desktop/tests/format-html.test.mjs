import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";

const { formatHtml } = await loadTypescript("../src/domain/formatHtml.ts");

test("indenta HTML compacto y conserva los atributos entrecomillados", () => {
  const html = '<!doctype html><html><body><p title="a > b">Hola</p><br><p>Mundo</p></body></html>';
  assert.equal(formatHtml(html), '<!doctype html>\n<html>\n  <body>\n    <p title="a > b">Hola</p>\n    <br>\n    <p>Mundo</p>\n  </body>\n</html>');
});

test("preserva el contenido literal de script, style, pre y textarea", () => {
  const blocks = ['<script>if (a < b) {\n  foo();\n}</script>', '<style>p { color: red; }</style>', '<pre>  a\n  b  </pre>', '<textarea>  texto\n  literal </textarea>'];
  const formatted = formatHtml(`<main>${blocks.join("")}</main>`);
  for (const block of blocks) assert.ok(formatted.includes(block), block);
});

test("acepta contenido vacío y comentarios", () => {
  assert.equal(formatHtml(""), "");
  assert.equal(formatHtml("<main><!-- hola --><p>ok</p></main>"), "<main>\n  <!-- hola -->\n  <p>ok</p>\n</main>");
});
