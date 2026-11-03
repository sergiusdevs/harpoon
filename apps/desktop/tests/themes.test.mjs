import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { loadTypescript } from './load-typescript.mjs';
const { themes, resolveTheme } = await loadTypescript('../src/domain/themes.ts');
const css = readFileSync(new URL('../src/app.css', import.meta.url), 'utf8');

test('cada tema tiene sus colores en app.css', () => {
  for (const { id } of themes) {
    if (id === 'system' || id === 'dark') continue; // dark = :root; system se resuelve
    assert.match(css, new RegExp(`:root\\[data-theme="${id}"\\] \\{[^}]*--bg:`), id);
  }
});

test('ids únicos y temas claros con color-scheme light', () => {
  assert.equal(new Set(themes.map((t) => t.id)).size, themes.length);
  for (const t of themes.filter((t) => t.kind === 'light' && t.id !== 'light')) {
    assert.match(css, new RegExp(`:root\\[data-theme="${t.id}"\\] \\{\\s*color-scheme: light`), t.id);
  }
});

test('automático sigue al sistema', () => {
  assert.equal(resolveTheme('system', true), 'dark');
  assert.equal(resolveTheme('system', false), 'light');
  assert.equal(resolveTheme('nord', false), 'nord');
});
