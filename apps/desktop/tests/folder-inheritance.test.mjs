import test from 'node:test';
import assert from 'node:assert/strict';
import { loadTypescript } from './load-typescript.mjs';
const { inheritedAuth, inheritedHeaders } = await loadTypescript('../src/domain/folderInheritance.ts');
const { runtimeHeaders, disabledRuntime } = await loadTypescript('../src/domain/runtimeHeaders.ts');

const col = {
  meta: { auth: { type: 'basic', username: 'u', password: 'p' } },
  folder_configs: {
    api: { headers: { 'X-Team': 'core', Accept: 'application/json' }, auth: { type: 'bearer', token: 't' } },
    'api/admin': { headers: { 'x-team': 'admin' }, auth: { type: 'inherit' } },
  },
};

test('cabeceras de fuera hacia dentro, la interior sustituye sin distinguir mayúsculas', () => {
  const rows = inheritedHeaders(col, ['api', 'api/admin']);
  assert.deepEqual(rows.map((r) => [r.name, r.value, r.source]), [['Accept', 'application/json', 'Carpeta api'], ['x-team', 'admin', 'Carpeta api/admin']]);
});

test('auth de la carpeta más cercana o de la colección', () => {
  assert.equal(inheritedAuth(col, ['api', 'api/admin']).source, 'carpeta api');
  assert.equal(inheritedAuth(col, []).source, 'la colección');
});

test('las cabeceras de carpeta aparecen en Headers y se pueden desactivar', () => {
  const request = { url: 'https://x.test', headers: {}, disabled_headers: ['X-TEAM'], auth: { type: 'inherit' }, body: { type: 'none' } };
  const folder = inheritedHeaders(col, ['api', 'api/admin']);
  const rows = runtimeHeaders(request, inheritedAuth(col, ['api', 'api/admin']), '', folder);
  assert.deepEqual(rows.slice(0, 3).map((r) => r.name), ['Accept', 'x-team', 'Authorization']);
  assert.equal(rows.find((r) => r.name === 'Authorization').source, 'Auth heredada de carpeta api');
  assert.deepEqual(disabledRuntime(request, folder), ['X-TEAM']);
});
