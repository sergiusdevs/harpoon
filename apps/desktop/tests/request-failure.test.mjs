import test from 'node:test';
import assert from 'node:assert/strict';
import { loadTypescript } from './load-typescript.mjs';
const { requestFailure } = await loadTypescript('../src/domain/requestFailure.ts');
test('conserva códigos y mensajes estructurados del backend', () => {
  const failure = { code: 'cancelled', title: 'Solicitud cancelada', message: 'Interrumpida', detail: '' };
  assert.deepEqual(requestFailure(failure), failure);
});
test('presenta causas comunes sin perder los detalles técnicos', () => {
  for (const [message, code] of [['variable sin definir: token', 'variable'], ['Connection refused', 'connection'], ['invalid peer certificate', 'tls'], ['tiempo agotado tras 30s', 'timeout']]) {
    const failure = requestFailure(new Error(message)); assert.equal(failure.code, code); assert.equal(failure.detail, message); assert.ok(failure.title && failure.message);
  }
});
