import { buildSchema, getOperationAST, getVariableValues, Kind, parse, validate, type GraphQLSchema } from "graphql";
import { getAutocompleteSuggestions, Position } from "graphql-language-service";

export type GraphqlIssue = { message: string; line?: number; column?: number };
let cachedText = "";
let cachedSchema: GraphQLSchema | undefined;
function schemaFor(text: string): GraphQLSchema {
  if (cachedSchema && text === cachedText) return cachedSchema;
  const schema = buildSchema(text);
  cachedSchema = schema; cachedText = text;
  return schema;
}
const issue = (error: unknown): GraphqlIssue => {
  const value = error as { message?: string; locations?: { line: number; column: number }[] };
  return { message: value.message ?? String(error), ...value.locations?.[0] };
};

export function validateGraphql(query: string, variables: string, operationName = "", schemaText = ""): GraphqlIssue[] {
  let values: Record<string, unknown>;
  try {
    const parsed = JSON.parse(variables.trim() || "{}");
    if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") return [{ message: "Las variables deben ser un objeto JSON." }];
    values = parsed;
  } catch { return [{ message: "Las variables contienen JSON no válido." }]; }
  try {
    const document = parse(query);
    if (document.definitions.some((definition) => definition.kind !== Kind.OPERATION_DEFINITION && definition.kind !== Kind.FRAGMENT_DEFINITION)) return [{ message: "La consulta debe contener operaciones o fragmentos, no definiciones de esquema." }];
    const operation = getOperationAST(document, operationName.trim() || undefined);
    if (!operation) return [{ message: operationName.trim() ? `No existe la operación ${operationName.trim()}.` : "Indica el nombre de operación cuando hay varias consultas." }];
    const missing = (operation.variableDefinitions ?? []).filter((definition) => definition.type.kind === Kind.NON_NULL_TYPE && !definition.defaultValue && !(definition.variable.name.value in values));
    if (missing.length) return missing.map((definition) => ({ message: `Falta la variable obligatoria $${definition.variable.name.value}.` }));
    if (!schemaText.trim()) return [];
    const schema = schemaFor(schemaText);
    const errors = validate(schema, document).map(issue);
    if (!JSON.stringify(values).includes("{{")) errors.push(...(getVariableValues(schema, operation.variableDefinitions ?? [], values).errors ?? []).map(issue));
    return errors;
  } catch (error) { return [issue(error)]; }
}

export function graphqlCompletions(schemaText: string, query: string, offset: number) {
  if (!schemaText.trim()) return ["query", "mutation", "subscription", "fragment", "on"].map((label) => ({ label, type: "keyword" }));
  try {
    const lines = query.slice(0, offset).split("\n");
    return getAutocompleteSuggestions(schemaFor(schemaText), query, new Position(lines.length - 1, lines.at(-1)!.length))
      .map((item) => ({ label: item.label, detail: item.detail ?? undefined, info: typeof item.documentation === "string" ? item.documentation : undefined, type: "property" }));
  } catch { return []; }
}

export function graphqlResponseErrors(body: string): { message: string; path?: string }[] {
  try {
    const value = JSON.parse(body);
    if (!Array.isArray(value?.errors)) return [];
    return value.errors.map((error: { message?: unknown; path?: unknown[] }) => ({ message: typeof error.message === "string" ? error.message : "Error GraphQL", path: Array.isArray(error.path) ? error.path.join(".") : undefined }));
  } catch { return []; }
}
