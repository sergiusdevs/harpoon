import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";
const { newTypedRequest, convertRequestType } = await loadTypescript('../src/domain/types.ts');
test('convertir a WS avisa del cuerpo y scripts y mantiene auth y cabeceras', () => {
  const source = newTypedRequest('Request', 'http', 'POST'); source.url = 'https://localhost/api'; source.body = { type: 'json', json: '{"a":1}' }; source.scripts.pre = 'console.log(1)'; source.headers.Authorization = 'Bearer demo';
  const { request, warnings } = convertRequestType(source, 'websocket');
  assert.equal(request.url, 'wss://localhost/api'); assert.equal(request.body.type, 'none'); assert.equal(request.headers.Authorization, 'Bearer demo'); assert.equal(request.scripts.pre, source.scripts.pre); assert.equal(warnings.length, 2); assert.equal(source.body.type, 'json');
});
test('GraphQL a HTTP conserva consulta, variables y operación en JSON', () => {
  const source = newTypedRequest('Query', 'graphql'); source.body = { type: 'graphql', query: 'query G { greeting }', variables: '{"name":"Ada"}', operation_name: 'G' };
  const converted = convertRequestType(source, 'http'); assert.equal(converted.request.body.type, 'json'); assert.deepEqual(JSON.parse(converted.request.body.json), { query: 'query G { greeting }', variables: { name: 'Ada' }, operationName: 'G' });
  assert.throws(() => convertRequestType({ ...source, body: { ...source.body, variables: '{' } }, 'http'), /Corrige/);
});
test('salir de SSE retira solo el Accept de eventos', () => {
  const source = newTypedRequest('Events', 'sse'); source.headers.accept = 'application/json, text/event-stream'; delete source.headers.Accept;
  const { request } = convertRequestType(source, 'http'); assert.equal(request.headers.accept, 'application/json');
});
