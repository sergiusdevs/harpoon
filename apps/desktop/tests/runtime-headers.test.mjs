import test from 'node:test';
import assert from 'node:assert/strict';
import { loadTypescript } from './load-typescript.mjs';
const { runtimeHeaders, disabledRuntime } = await loadTypescript('../src/domain/runtimeHeaders.ts');

const request = (extra = {}) => ({ name: 'r', method: 'GET', url: 'https://api.test/x', headers: {}, query: {}, disabled_query: [], disabled_headers: [], auth: { type: 'inherit' }, body: { type: 'none' }, ...extra });
const names = (rows) => rows.map((r) => r.name);

test('incluye auth heredada, Content-Type del cuerpo y las del cliente HTTP', () => {
  const rows = runtimeHeaders(request({ body: { type: 'json', json: '{}' } }), { auth: { type: 'bearer', token: 't' }, source: 'la colección' }, 'https://api.test/x');
  assert.deepEqual(names(rows), ['Authorization', 'Content-Type', 'User-Agent', 'Accept', 'Accept-Encoding', 'Host']);
  assert.equal(rows.find((r) => r.name === 'Host').value, 'api.test');
  assert.deepEqual(rows.filter((r) => r.toggleable).map((r) => r.name), ['User-Agent']);
});

test('una cabecera propia activa sustituye a la del runtime; desactivada no', () => {
  const own = runtimeHeaders(request({ headers: { accept: 'application/json' } }), { auth: { type: 'none' }, source: 'la colección' }, '');
  assert.equal(own.find((r) => r.name === 'Accept').overridden, true);
  const off = runtimeHeaders(request({ headers: { accept: 'x' }, disabled_headers: ['Accept'] }), { auth: { type: 'none' }, source: 'la colección' }, '');
  assert.equal(off.find((r) => r.name === 'Accept').overridden, false);
});

test('distingue User-Agent desactivado del runtime de filas de la tabla', () => {
  assert.deepEqual(disabledRuntime(request({ disabled_headers: ['user-agent', 'X-Debug'] })), ['user-agent']);
  assert.deepEqual(disabledRuntime(request({ headers: { 'User-Agent': 'a' }, disabled_headers: ['User-Agent'] })), []);
});
