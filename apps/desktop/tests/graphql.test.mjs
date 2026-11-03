import test from "node:test";
import assert from "node:assert/strict";
import { loadTypescript } from "./load-typescript.mjs";
const { validateGraphql, graphqlCompletions, graphqlResponseErrors } = await loadTypescript("../src/domain/graphql.ts");
const schema = 'type Query { user(id: ID!): User, greeting(name: String!): String! } type User { id: ID!, name: String! }';
test("valida sintaxis y JSON sin necesitar un esquema", () => {
  assert.equal(validateGraphql('{ user { name } }', '{}').length, 0);
  assert.match(validateGraphql('{ user {', '{}')[0].message, /Syntax/);
  assert.match(validateGraphql('{ user { name } }', '[]')[0].message, /objeto JSON/);
  assert.match(validateGraphql('{ user { name } }', '{')[0].message, /JSON no válido/);
});
test("selecciona operaciones y detecta variables obligatorias ausentes", () => {
  assert.match(validateGraphql('query One { greeting(name:"Ada") } query Two { greeting(name:"Grace") }', '{}')[0].message, /nombre de operación/);
  assert.match(validateGraphql('query One { user { id } }', '{}', 'Missing')[0].message, /No existe/);
  assert.match(validateGraphql('query User($id: ID!) { user(id:$id) { name } }', '{}')[0].message, /\$id/);
});
test("usa SDL para validar campos y tipos de variables", () => {
  assert.deepEqual(validateGraphql('query User($id: ID!) { user(id:$id) { name } }', '{"id":1}', 'User', schema), []);
  assert.match(validateGraphql('{ unknownField }', '{}', '', schema)[0].message, /unknownField/);
  assert.match(validateGraphql('query G($name: String!) { greeting(name:$name) }', '{"name":42}', 'G', schema)[0].message, /String/);
  assert.deepEqual(validateGraphql('query G($name: String!) { greeting(name:$name) }', '{"name":"{{name}}"}', 'G', schema), []);
});
test("ofrece campos del esquema como sugerencias", () => {
  const query = '{ user(id:1) { na';
  assert.ok(graphqlCompletions(schema, query, query.length).some((item) => item.label === 'name'));
});
test("muestra errores GraphQL aunque HTTP sea 200", () => {
  assert.deepEqual(graphqlResponseErrors('{"data":null,"errors":[{"message":"Denied","path":["user","name"]}]}'), [{ message: 'Denied', path: 'user.name' }]);
  assert.deepEqual(graphqlResponseErrors('not JSON'), []);
});
