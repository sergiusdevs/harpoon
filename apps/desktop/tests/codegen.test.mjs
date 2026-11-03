import test from 'node:test';
import assert from 'node:assert/strict';
import { loadTypescript } from './load-typescript.mjs';
const { generateCode, LANGUAGES } = await loadTypescript('../src/domain/codegen.ts');

const settings = { timeout_ms: 30000, follow_redirects: true, insecure: false };
const json = {
  request: { method: 'POST', url: 'https://api.test/users', query: { page: '2' }, headers: { 'Content-Type': 'application/json', Authorization: 'Bearer t"k' }, body: '{"name":"Ada"}' },
  digest: null,
  settings,
};

test('todos los lenguajes generan código con URL, método, cabeceras y cuerpo', () => {
  for (const { id } of LANGUAGES) {
    const code = generateCode(id, json);
    assert.match(code, /https:\/\/api\.test\/users\?page=2/, id);
    assert.match(code, /POST|post/, id);
    assert.ok(code.includes('Authorization') && code.includes('Bearer t\\"k') || id === 'curl', id);
    assert.ok(code.includes('name') && code.includes('Ada'), id);
  }
});

test('python usa HTTPDigestAuth y respeta TLS y redirecciones', () => {
  const code = generateCode('python', { ...json, digest: { username: 'u', password: 'p' }, settings: { ...settings, follow_redirects: false, insecure: true } });
  assert.match(code, /from requests\.auth import HTTPDigestAuth/);
  assert.match(code, /auth=HTTPDigestAuth\("u", "p"\)/);
  assert.match(code, /allow_redirects=False/);
  assert.match(code, /verify=False/);
});

test('multipart y binario', () => {
  const multipart = { ...json, request: { ...json.request, body: null, headers: { 'Content-Type': 'multipart/form-data' }, multipart: [{ type: 'text', name: 'a', value: '1' }, { type: 'file', name: 'doc', path: 'docs/a.pdf' }] } };
  const fetch = generateCode('fetch', multipart);
  assert.match(fetch, /new FormData\(\)/);
  assert.match(fetch, /openAsBlob\("docs\/a\.pdf"\)/);
  assert.doesNotMatch(fetch, /"Content-Type"/, 'el boundary lo pone FormData');
  assert.match(generateCode('go', multipart), /form\.FormDataContentType\(\)/);
  assert.match(generateCode('csharp', multipart), /MultipartFormDataContent/);
  const binary = { ...json, request: { ...json.request, body: null, binary_path: 'data.bin' } };
  assert.match(generateCode('java', binary), /ofFile\(Path\.of\("data\.bin"\)\)/);
  assert.match(generateCode('python', binary), /data=open\("data\.bin", "rb"\)/);
});

test('avisa de lo que el lenguaje no traduce', () => {
  const code = generateCode('fetch', { ...json, digest: { username: 'u', password: 'p' }, settings: { ...settings, proxy: 'http://proxy:8080' } });
  assert.match(code, /^\/\/ Auth Digest/m);
  assert.match(code, /proxy http:\/\/proxy:8080/);
});
