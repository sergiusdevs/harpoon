import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";
const { renderMarkdown } = await loadTypescript("../src/domain/markdown.ts");
const payloads = [
  '<img src=x onerror=alert(1)>', '<script>alert(1)</script>', '[x](javascript:alert(1))', '[x](JaVaScRiPt:alert(1))',
  '[x]( javascript:alert(1))', '[x](data:text/html,<script>alert(1)</script>)', '![a](javascript:alert(1))',
  '[x](https://a.com" onmouseover="alert(1))', '`<b onclick=1>`', '```\n<script>x</script>\n```', '| a | b |\n|---|---|\n| <img src=x onerror=1> | c |',
  '- [ ] <svg onload=alert(1)>', '> <iframe src=x>', '# <a href="javascript:1">h</a>', '[x](https://a.com\'onclick=\'1)', '**<style>*{}</style>**',
  '[x](vbscript:msgbox)', '[x](&#106;avascript:alert(1))', '<a href=javascript:1>x</a>',
];
test("markdown no deja pasar HTML activo", () => {
  for (const p of payloads) {
    const html = renderMarkdown(p);
    const allowed = new Set(["p","a","strong","em","del","code","pre","ul","ol","li","input","blockquote","hr","table","thead","tbody","tr","th","td","h1","h2","h3","h4","h5","h6","br","span"]);
    for (const [tag] of html.matchAll(/<[^>]*>/g)) {
      const name = /^<\/?([a-z0-9]+)/i.exec(tag)?.[1]?.toLowerCase();
      assert.ok(name && allowed.has(name), `etiqueta ${tag} en ${p}`);
      assert.doesNotMatch(tag, /\son\w+\s*=/i, `${tag} en ${p}`);
      const href = /href="([^"]*)"/.exec(tag)?.[1];
      if (href) assert.match(href, /^(https?:|mailto:)/i, `${tag} en ${p}`);
      assert.ok((tag.match(/"/g) ?? []).length % 2 === 0, `comillas en ${tag}`);
    }
  }
});
