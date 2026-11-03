import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";

const { folderChain } = await loadTypescript("../src/domain/paths.ts");

test("devuelve las carpetas de fuera hacia dentro, sin incluir la solicitud", () => {
  assert.deepEqual(folderChain("api/users/list"), ["api", "api/users"]);
  assert.deepEqual(folderChain("api/list"), ["api"]);
  assert.deepEqual(folderChain("health"), []);
});
