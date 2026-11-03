import test from 'node:test';
import assert from 'node:assert/strict';
import { loadTypescript } from './load-typescript.mjs';
const { renderMarkdown, htmlToMarkdown } = await loadTypescript('../src/domain/markdown.ts');

const txt = (text) => ({ nodeType: 3, nodeName: '#text', textContent: text, childNodes: [] });
const el = (tag, attrs, ...kids) => ({
  nodeType: 1, nodeName: tag.toUpperCase(), childNodes: kids.map((k) => (typeof k === 'string' ? txt(k) : k)),
  get textContent() { return this.childNodes.map((c) => c.textContent).join(''); },
  getAttribute: (name) => (attrs && name in attrs ? attrs[name] : null),
});

test('renderiza títulos, énfasis, código y listas', () => {
  assert.equal(renderMarkdown('# Hola *mundo*'), '<h1>Hola <em>mundo</em></h1>');
  assert.equal(renderMarkdown('**a** ~~b~~ `c*d`'), '<p><strong>a</strong> <del>b</del> <code>c*d</code></p>');
  assert.equal(renderMarkdown('- uno\n- dos'), '<ul><li>uno</li><li>dos</li></ul>');
  assert.equal(renderMarkdown('1. uno\n2. dos'), '<ol><li>uno</li><li>dos</li></ol>');
  assert.match(renderMarkdown('- [x] hecho\n- [ ] pendiente'), /<li class="task"><input type="checkbox" disabled checked> hecho<\/li><li class="task"><input type="checkbox" disabled> pendiente/);
  assert.equal(renderMarkdown('```js\nif (a < b) {}\n```'), '<pre data-lang="js"><code>if (a &lt; b) {}</code></pre>');
  assert.equal(renderMarkdown('snake_case_name'), '<p>snake_case_name</p>');
});

test('tablas simples', () => {
  assert.equal(renderMarkdown('| a | b |\n| --- | --: |\n| 1 | 2 |'), '<table><thead><tr><th>a</th><th style="text-align:right">b</th></tr></thead><tbody><tr><td>1</td><td style="text-align:right">2</td></tr></tbody></table>');
});

test('escapa HTML y solo permite enlaces http(s) y mailto', () => {
  assert.equal(renderMarkdown('<img src=x onerror=alert(1)>'), '<p>&lt;img src=x onerror=alert(1)&gt;</p>');
  assert.equal(renderMarkdown('[x](javascript:alert)'), '<p>x</p>');
  assert.equal(renderMarkdown('[web](https://a.com/?q="x")'), '<p>web</p>');
  assert.equal(renderMarkdown('[web](https://a.com)'), '<p><a href="https://a.com" target="_blank" rel="noopener noreferrer">web</a></p>');
  assert.match(renderMarkdown('[m](mailto:a@b.c)'), /href="mailto:a@b.c"/);
});

test('convierte el DOM del editor enriquecido en Markdown', () => {
  const root = el('div', null,
    el('h2', null, 'Título'),
    el('p', null, 'Texto ', el('strong', null, 'fuerte '), el('em', null, 'cursiva'), ' y ', el('a', { href: 'https://x.dev' }, 'enlace'), ' ', el('a', { href: 'javascript:x' }, 'malo')),
    el('ul', null, el('li', null, 'uno'), el('li', null, 'dos', el('ul', null, el('li', null, 'anidado')))),
    el('ul', null, el('li', null, el('input', { checked: '' }), ' hecho')),
    el('pre', null, el('code', null, 'a *b*')),
    el('table', null, el('tbody', null, el('tr', null, el('th', null, 'k'), el('th', null, 'v')), el('tr', null, el('td', null, 'a|b'), el('td', null, '2')))),
    el('p', null, '# no es título'),
  );
  assert.equal(htmlToMarkdown(root), [
    '## Título', 'Texto **fuerte** *cursiva* y [enlace](https://x.dev) malo', '- uno\n- dos\n   - anidado', '- [x] hecho', '```\na *b*\n```',
    '| k | v |\n| --- | --- |\n| a\\|b | 2 |', '\\# no es título',
  ].join('\n\n'));
  assert.equal(renderMarkdown('\\# no es título'), '<p># no es título</p>');
  assert.equal(htmlToMarkdown(el('div', null, el('p', null, '1. no lista *x*'))), '1\\. no lista \\*x\\*');
  assert.equal(renderMarkdown('1\\. no lista \\*x\\*'), '<p>1. no lista *x*</p>');
});
