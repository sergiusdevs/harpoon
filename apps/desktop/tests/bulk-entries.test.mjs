import test from 'node:test';
import assert from 'node:assert/strict';
import { loadTypescript } from './load-typescript.mjs';
const { parseBulkEntries } = await loadTypescript('../src/domain/bulkEntries.ts');
test('edición en bloque admite tabuladores, descripciones y valores con dos puntos', () => {
  assert.deepEqual(parseBulkEntries('page\t0\tPágina inicial\nReferer: https://localhost/path\na=b=c'), [{ k: 'page', v: '0', d: 'Página inicial' }, { k: 'Referer', v: 'https://localhost/path', d: '' }, { k: 'a', v: 'b=c', d: '' }]);
});
test('rechaza filas inválidas y nombres duplicados antes de modificar la tabla', () => {
  assert.throws(() => parseBulkEntries('page: 0\npage: 1'), /repetido/);
  assert.throws(() => parseBulkEntries('page: 0\nincorrecto'), /Línea 2/);
  assert.throws(() => parseBulkEntries('\tvalue'), /falta el nombre/);
});
test('permite borrar todas las filas o conservar un valor vacío', () => {
  assert.deepEqual(parseBulkEntries(''), []); assert.deepEqual(parseBulkEntries('token:\nkey\t\tDescripción'), [{ k: 'token', v: '', d: '' }, { k: 'key', v: '', d: 'Descripción' }]);
});
test('las líneas con // quedan desactivadas y se serializan igual', async () => {
  const { serializeBulkEntries } = await loadTypescript('../src/domain/bulkEntries.ts');
  assert.deepEqual(parseBulkEntries('Accept: text/html\n// X-Debug: 1\n  //Origin:null'), [{ k: 'Accept', v: 'text/html', d: '' }, { k: 'X-Debug', v: '1', d: '', disabled: true }, { k: 'Origin', v: 'null', d: '', disabled: true }]);
  assert.equal(serializeBulkEntries({ Accept: 'text/html', 'X-Debug': '1' }, (k) => k === 'X-Debug'), 'Accept: text/html\n//X-Debug: 1');
  assert.deepEqual(parseBulkEntries('//'), []);
});
