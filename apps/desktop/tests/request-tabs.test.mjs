import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";
const { emptyTabs, activateTab, remapTabs, pruneTabs } = await loadTypescript('../src/domain/requestTabs.ts');
const requests = { one: { name: 'One', method: 'GET', url: '/one' }, two: { name: 'Two', method: 'POST', url: '/two' } };
test('cambiar de pestaña conserva borrador, cambios y respuesta sin escribir en disco', () => {
  const tabs = emptyTabs();
  const first = activateTab(tabs, null, null, 'one', requests);
  first.draft.url = '/modified'; first.dirty = true; first.exchange = { response: { status: 200 } };
  const second = activateTab(tabs, 'one', first, 'two', requests);
  const restored = activateTab(tabs, 'two', second, 'one', requests);
  assert.equal(restored.draft.url, '/modified'); assert.equal(restored.dirty, true); assert.equal(restored.exchange.response.status, 200);
  assert.equal(requests.one.url, '/one'); assert.deepEqual(tabs.opened, ['one', 'two']);
});
test('renombrar carpetas conserva pestañas y cambios pendientes', () => {
  const tabs = emptyTabs(); tabs.opened = ['users/one', 'two']; tabs.views['users/one'] = { draft: requests.one, dirty: true };
  remapTabs(tabs, 'users', 'people', true);
  assert.deepEqual(tabs.opened, ['people/one', 'two']); assert.equal(tabs.views['people/one'].dirty, true);
});
test('eliminar solicitudes retira únicamente sus pestañas y borradores', () => {
  const tabs = emptyTabs(); tabs.opened = ['one', 'two']; tabs.views.one = { dirty: true }; tabs.views.two = { dirty: true };
  pruneTabs(tabs, { two: requests.two }); assert.deepEqual(tabs.opened, ['two']); assert.equal(tabs.views.one, undefined); assert.equal(tabs.views.two.dirty, true);
});
