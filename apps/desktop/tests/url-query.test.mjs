import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";

const { splitUrl, composeUrl, applyUrlInput } = await loadTypescript("../src/domain/urlQuery.ts");
const empty = { url: "", query: {}, disabled_query: [] };

test("pegar una URL con parámetros los pasa a Params", () => {
  const url = "https://www.carrefour.es/cloud-api/categories-api/v1/categories/menu?sale_point=005290&current_category=cat20001";
  assert.deepEqual(applyUrlInput(url, empty), {
    url: "https://www.carrefour.es/cloud-api/categories-api/v1/categories/menu",
    query: { sale_point: "005290", current_category: "cat20001" },
    disabled_query: [],
  });
});

test("ida y vuelta conserva el texto, variables y fragmento", () => {
  for (const text of ["{{base_url}}/a?x={{id}}&flag&q=a=b#top", "https://h/p", "https://h/p?a=1&b=", "{{host}}/s?{{k}}=v"]) {
    const s = applyUrlInput(text, empty);
    assert.equal(composeUrl(s.url, s.query, s.disabled_query), text.replace("&b=", "&b"), text);
  }
});

test("? dentro de una variable no parte la URL", () => {
  assert.deepEqual(splitUrl("{{base?x}}/p"), { base: "{{base?x}}/p", pairs: [], hasQuery: false });
});

test("los parámetros desactivados se conservan y no salen en la barra", () => {
  const prev = { url: "https://h/p", query: { a: "1", debug: "true" }, disabled_query: ["debug"] };
  assert.equal(composeUrl(prev.url, prev.query, prev.disabled_query), "https://h/p?a=1");
  const next = applyUrlInput("https://h/p?a=2&c=3", prev);
  assert.deepEqual(next, { url: "https://h/p", query: { a: "2", c: "3", debug: "true" }, disabled_query: ["debug"] });
  assert.deepEqual(applyUrlInput("https://h/p?debug=false", prev).disabled_query, []);
});

test("URLs antiguas con query en la url se recomponen sin duplicar", () => {
  assert.equal(composeUrl("https://h/p?old=1", { a: "2" }), "https://h/p?old=1&a=2");
});
