// Código listo para copiar a partir de una solicitud preparada (variables,
// auth y herencia ya resueltas en el backend). cURL vive en `curlExport.ts`.
import { buildCurl, type CurlPrepared } from "./curlExport";

export const LANGUAGES = [
  { id: "curl", name: "cURL", hint: "sh, zsh y bash" },
  { id: "fetch", name: "JavaScript · fetch", hint: "Node 18+ o navegador" },
  { id: "axios", name: "JavaScript · axios", hint: "Node con axios" },
  { id: "python", name: "Python · requests", hint: "pip install requests" },
  { id: "go", name: "Go · net/http", hint: "Solo biblioteca estándar" },
  { id: "java", name: "Java · HttpClient", hint: "Java 11+" },
  { id: "csharp", name: "C# · HttpClient", hint: ".NET 6+" },
] as const;

export type Language = (typeof LANGUAGES)[number]["id"];

/** Literal de cadena con comillas dobles; válido en JS, Python, Go, Java y C#. */
const str = (value: string) => JSON.stringify(value);

function fullUrl(p: CurlPrepared): string {
  const { url: raw, query } = p.request;
  const fragment = raw.indexOf("#");
  let url = fragment < 0 ? raw : raw.slice(0, fragment);
  const qs = new URLSearchParams(Object.entries(query)).toString();
  if (qs) url += (url.includes("?") ? (url.endsWith("?") || url.endsWith("&") ? "" : "&") : "?") + qs;
  return url;
}

const fileName = (path: string) => path.split(/[\\/]/).pop() || "archivo";
const contentType = (h: Record<string, string>) => Object.entries(h).find(([k]) => k.toLowerCase() === "content-type")?.[1];

/** Avisos comunes: lo que el lenguaje de destino no traduce directamente. */
function notes(p: CurlPrepared, comment: string, opts: { digest?: boolean } = {}): string[] {
  const out: string[] = [];
  const s = p.settings;
  if (p.digest && !opts.digest) out.push(`${comment} Auth Digest (${p.digest.username}): este cliente no la resuelve solo; usa una librería o cURL.`);
  if (s.proxy) out.push(`${comment} La colección usa el proxy ${s.proxy}.`);
  if (s.insecure) out.push(`${comment} La colección acepta certificados TLS inválidos.`);
  if (s.ca_cert || s.client_cert) out.push(`${comment} La colección usa certificados propios (${[s.ca_cert, s.client_cert].filter(Boolean).join(", ")}).`);
  return out;
}

function js(p: CurlPrepared, axios: boolean): string {
  const r = p.request;
  const lines: string[] = notes(p, "//");
  const usesFiles = !!r.multipart?.some((part) => part.type === "file") || !!r.binary_path;
  if (axios) lines.push('import axios from "axios";');
  if (usesFiles) lines.push(`import { ${r.binary_path ? "readFile" : "openAsBlob"} } from "node:fs${r.binary_path ? "/promises" : ""}";`);
  if (lines.length) lines.push("");
  let body: string | null = null;
  if (r.multipart) {
    lines.push("const form = new FormData();");
    for (const part of r.multipart) {
      if (part.type === "text") lines.push(`form.append(${str(part.name)}, ${str(part.value)});`);
      else lines.push(`form.append(${str(part.name)}, await openAsBlob(${str(part.path)}${part.content_type ? `, { type: ${str(part.content_type)} }` : ""}), ${str(fileName(part.path))});`);
    }
    lines.push("");
    body = "form";
  } else if (r.binary_path) body = `await readFile(${str(r.binary_path)})`;
  else if (r.body != null) body = str(r.body);
  const headers = Object.entries(r.headers).filter(([k]) => !(r.multipart && k.toLowerCase() === "content-type"));
  const headerBlock = headers.length ? `  headers: {\n${headers.map(([k, v]) => `    ${str(k)}: ${str(v)},`).join("\n")}\n  },\n` : "";
  if (axios) {
    lines.push("const response = await axios({");
    lines.push(`  method: ${str(r.method.toLowerCase())},`);
    lines.push(`  url: ${str(fullUrl(p))},`);
    if (headerBlock) lines.push(headerBlock.trimEnd());
    if (body) lines.push(`  data: ${body},`);
    if (p.settings.timeout_ms > 0) lines.push(`  timeout: ${p.settings.timeout_ms},`);
    if (!p.settings.follow_redirects) lines.push("  maxRedirects: 0,");
    lines.push("  validateStatus: () => true,");
    lines.push("});");
    lines.push("console.log(response.status, response.data);");
  } else {
    lines.push(`const response = await fetch(${str(fullUrl(p))}, {`);
    lines.push(`  method: ${str(r.method)},`);
    if (headerBlock) lines.push(headerBlock.trimEnd());
    if (body) lines.push(`  body: ${body},`);
    if (!p.settings.follow_redirects) lines.push('  redirect: "manual",');
    lines.push("});");
    lines.push("console.log(response.status, await response.text());");
  }
  return lines.join("\n");
}

function python(p: CurlPrepared): string {
  const r = p.request;
  const lines = [...notes(p, "#", { digest: true }), "import requests"];
  if (p.digest) lines.push("from requests.auth import HTTPDigestAuth");
  lines.push("");
  const args = [`    ${str(r.method)},`, `    ${str(fullUrl(p))},`];
  const headers = Object.entries(r.headers).filter(([k]) => !(r.multipart && k.toLowerCase() === "content-type"));
  if (headers.length) args.push(`    headers={\n${headers.map(([k, v]) => `        ${str(k)}: ${str(v)},`).join("\n")}\n    },`);
  if (r.multipart) {
    const text = r.multipart.filter((part) => part.type === "text");
    const files = r.multipart.filter((part) => part.type === "file");
    if (text.length) args.push(`    data={${text.map((part) => `${str(part.name)}: ${str(part.type === "text" ? part.value : "")}`).join(", ")}},`);
    if (files.length) args.push(`    files={${files.map((part) => part.type === "file" ? `${str(part.name)}: (${str(fileName(part.path))}, open(${str(part.path)}, "rb")${part.content_type ? `, ${str(part.content_type)}` : ""})` : "").join(", ")}},`);
  } else if (r.binary_path) args.push(`    data=open(${str(r.binary_path)}, "rb"),`);
  else if (r.body != null) args.push(`    data=${str(r.body)}.encode("utf-8"),`);
  if (p.digest) args.push(`    auth=HTTPDigestAuth(${str(p.digest.username)}, ${str(p.digest.password)}),`);
  if (p.settings.timeout_ms > 0) args.push(`    timeout=${p.settings.timeout_ms / 1000},`);
  if (!p.settings.follow_redirects) args.push("    allow_redirects=False,");
  if (p.settings.insecure) args.push("    verify=False,");
  else if (p.settings.ca_cert) args.push(`    verify=${str(p.settings.ca_cert)},`);
  if (p.settings.client_cert) args.push(`    cert=${p.settings.client_key ? `(${str(p.settings.client_cert)}, ${str(p.settings.client_key)})` : str(p.settings.client_cert)},`);
  if (p.settings.proxy) args.push(`    proxies={"http": ${str(p.settings.proxy)}, "https": ${str(p.settings.proxy)}},`);
  lines.push("response = requests.request(", ...args, ")", "print(response.status_code, response.text)");
  return lines.join("\n");
}

function go(p: CurlPrepared): string {
  const r = p.request;
  const imports = new Set(["fmt", "io", "net/http"]);
  const body: string[] = [];
  let reader = "nil";
  if (r.multipart) {
    imports.add("bytes"); imports.add("mime/multipart");
    body.push("\tvar buf bytes.Buffer", "\tform := multipart.NewWriter(&buf)");
    for (const part of r.multipart) {
      if (part.type === "text") body.push(`\tform.WriteField(${str(part.name)}, ${str(part.value)})`);
      else {
        imports.add("os");
        body.push(`\tif f, err := os.Open(${str(part.path)}); err == nil {`, `\t\tw, _ := form.CreateFormFile(${str(part.name)}, ${str(fileName(part.path))})`, "\t\tio.Copy(w, f)", "\t\tf.Close()", "\t}");
      }
    }
    body.push("\tform.Close()", "");
    reader = "&buf";
  } else if (r.binary_path) {
    imports.add("os");
    body.push(`\tfile, err := os.Open(${str(r.binary_path)})`, "\tif err != nil {", "\t\tpanic(err)", "\t}", "\tdefer file.Close()", "");
    reader = "file";
  } else if (r.body != null) {
    imports.add("strings");
    reader = `strings.NewReader(${str(r.body)})`;
  }
  body.push(`\treq, err := http.NewRequest(${str(r.method)}, ${str(fullUrl(p))}, ${reader})`, "\tif err != nil {", "\t\tpanic(err)", "\t}");
  for (const [k, v] of Object.entries(r.headers)) {
    if (r.multipart && k.toLowerCase() === "content-type") continue;
    body.push(`\treq.Header.Set(${str(k)}, ${str(v)})`);
  }
  if (r.multipart) body.push('\treq.Header.Set("Content-Type", form.FormDataContentType())');
  const client: string[] = [];
  if (p.settings.timeout_ms > 0) { imports.add("time"); client.push(`Timeout: ${p.settings.timeout_ms} * time.Millisecond`); }
  if (!p.settings.follow_redirects) client.push("CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }");
  body.push("", `\tclient := &http.Client{${client.join(", ")}}`, "\tres, err := client.Do(req)", "\tif err != nil {", "\t\tpanic(err)", "\t}", "\tdefer res.Body.Close()", "\tdata, _ := io.ReadAll(res.Body)", "\tfmt.Println(res.StatusCode, string(data))");
  return [...notes(p, "//"), "package main", "", "import (", ...[...imports].sort().map((i) => `\t${str(i)}`), ")", "", "func main() {", ...body, "}"].join("\n");
}

function java(p: CurlPrepared): string {
  const r = p.request;
  const lines = notes(p, "//");
  if (r.multipart) lines.push("// Multipart: HttpClient no lo construye solo; usa una librería (p. ej. Apache HttpClient 5) o cURL.");
  let publisher = "HttpRequest.BodyPublishers.noBody()";
  if (r.binary_path) publisher = `HttpRequest.BodyPublishers.ofFile(Path.of(${str(r.binary_path)}))`;
  else if (r.body != null) publisher = `HttpRequest.BodyPublishers.ofString(${str(r.body)})`;
  lines.push("import java.net.URI;", "import java.net.http.HttpClient;", "import java.net.http.HttpRequest;", "import java.net.http.HttpResponse;");
  if (r.binary_path) lines.push("import java.nio.file.Path;");
  if (p.settings.timeout_ms > 0) lines.push("import java.time.Duration;");
  lines.push("", "public class Main {", "    public static void main(String[] args) throws Exception {");
  const client = ["        HttpClient client = HttpClient.newBuilder()"];
  client.push(`            .followRedirects(HttpClient.Redirect.${p.settings.follow_redirects ? "NORMAL" : "NEVER"})`);
  if (p.settings.timeout_ms > 0) client.push(`            .connectTimeout(Duration.ofMillis(${p.settings.timeout_ms}))`);
  client.push("            .build();");
  lines.push(...client, "", `        HttpRequest request = HttpRequest.newBuilder(URI.create(${str(fullUrl(p))}))`);
  for (const [k, v] of Object.entries(r.headers)) lines.push(`            .header(${str(k)}, ${str(v)})`);
  lines.push(`            .method(${str(r.method)}, ${publisher})`, "            .build();", "", "        HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());", "        System.out.println(response.statusCode() + \" \" + response.body());", "    }", "}");
  return lines.join("\n");
}

function csharp(p: CurlPrepared): string {
  const r = p.request;
  const lines = [...notes(p, "//"), "using System.Net.Http.Headers;", ""];
  lines.push(`var handler = new HttpClientHandler { AllowAutoRedirect = ${p.settings.follow_redirects ? "true" : "false"} };`);
  lines.push(`using var client = new HttpClient(handler)${p.settings.timeout_ms > 0 ? ` { Timeout = TimeSpan.FromMilliseconds(${p.settings.timeout_ms}) }` : ""};`);
  lines.push(`using var request = new HttpRequestMessage(new HttpMethod(${str(r.method)}), ${str(fullUrl(p))});`);
  const type = contentType(r.headers);
  for (const [k, v] of Object.entries(r.headers)) {
    if (k.toLowerCase() === "content-type") continue;
    lines.push(`request.Headers.TryAddWithoutValidation(${str(k)}, ${str(v)});`);
  }
  if (r.multipart) {
    lines.push("var form = new MultipartFormDataContent();");
    for (const part of r.multipart) {
      if (part.type === "text") lines.push(`form.Add(new StringContent(${str(part.value)}), ${str(part.name)});`);
      else {
        lines.push(`var file_${part.name.replace(/\W/g, "_")} = new ByteArrayContent(File.ReadAllBytes(${str(part.path)}));`);
        if (part.content_type) lines.push(`file_${part.name.replace(/\W/g, "_")}.Headers.ContentType = MediaTypeHeaderValue.Parse(${str(part.content_type)});`);
        lines.push(`form.Add(file_${part.name.replace(/\W/g, "_")}, ${str(part.name)}, ${str(fileName(part.path))});`);
      }
    }
    lines.push("request.Content = form;");
  } else if (r.binary_path || r.body != null) {
    lines.push(r.binary_path ? `request.Content = new ByteArrayContent(File.ReadAllBytes(${str(r.binary_path)}));` : `request.Content = new StringContent(${str(r.body ?? "")});`);
    lines.push(type ? `request.Content.Headers.ContentType = MediaTypeHeaderValue.Parse(${str(type)});` : "request.Content.Headers.ContentType = null;");
  }
  lines.push("", "using var response = await client.SendAsync(request);", "Console.WriteLine($\"{(int)response.StatusCode} {await response.Content.ReadAsStringAsync()}\");");
  return lines.join("\n");
}

export function generateCode(language: Language, prepared: CurlPrepared): string {
  switch (language) {
    case "curl": return buildCurl(prepared);
    case "fetch": return js(prepared, false);
    case "axios": return js(prepared, true);
    case "python": return python(prepared);
    case "go": return go(prepared);
    case "java": return java(prepared);
    case "csharp": return csharp(prepared);
  }
}
