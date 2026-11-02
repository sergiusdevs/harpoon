import type { Method, Settings, MultipartPart } from "./types";

export interface CurlPrepared {
  request: { method: Method; url: string; headers: Record<string, string>; query: Record<string, string>; body?: string | null; multipart?: MultipartPart[] | null };
  digest: { username: string; password: string } | null;
  settings: Settings;
}

/** Genera argumentos para POSIX sin interpolación ni ejecución por la shell. */
export function buildCurl(prepared: CurlPrepared): string {
  const { request, settings, digest } = prepared;
  const quote = (value: string) => {
    if (value.includes("\0")) throw new Error("cURL no admite caracteres NUL en los argumentos");
    return `'${value.replaceAll("'", `'\\''`)}'`;
  };
  const fragment = request.url.indexOf("#");
  let url = fragment < 0 ? request.url : request.url.slice(0, fragment);
  const query = new URLSearchParams(Object.entries(request.query)).toString();
  if (query) url += (url.includes("?") ? (url.endsWith("?") || url.endsWith("&") ? "" : "&") : "?") + query;
  const args = ["curl", ...(request.method === "HEAD" ? ["--head"] : ["--request", quote(request.method)]), "--url", quote(url)];
  if (settings.follow_redirects) args.push("--location", "--max-redirs", "10");
  if (settings.timeout_ms > 0) args.push("--max-time", String(settings.timeout_ms / 1000));
  if (settings.insecure) args.push("--insecure");
  if (settings.proxy) args.push("--proxy", quote(settings.proxy));
  if (settings.ca_cert) args.push("--cacert", quote(settings.ca_cert));
  if (settings.client_cert) args.push("--cert", quote(settings.client_cert));
  if (settings.client_key) args.push("--key", quote(settings.client_key));
  if (digest) {
    if (digest.username.includes(":")) throw new Error("cURL no admite dos puntos en el nombre de usuario Digest");
    args.push("--digest", "--user", quote(`${digest.username}:${digest.password}`));
  }
  for (const [key, value] of Object.entries(request.headers)) {
    if (/[\r\n:]/.test(key) || /[\r\n]/.test(value)) throw new Error(`Cabecera inválida: ${key}`);
    args.push("--header", quote(value === "" ? `${key};` : `${key}: ${value}`));
  }
  if (request.multipart) {
    for (const part of request.multipart) {
      if (!part.name || /[=\r\n]/.test(part.name)) throw new Error("El nombre del campo multipart no se puede representar en cURL");
      if (part.type === "text") args.push("--form-string", quote(`${part.name}=${part.value}`));
      else {
        if (part.content_type && /[;\r\n]/.test(part.content_type)) throw new Error("Tipo de contenido multipart inválido");
        const path = part.path.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
        args.push("--form", quote(`${part.name}=@"${path}"${part.content_type ? `;type=${part.content_type}` : ""}`));
      }
    }
  }
  if (request.body !== undefined && request.body !== null) {
    if (!Object.keys(request.headers).some((name) => name.toLowerCase() === "content-type")) args.push("--header", quote("Content-Type:"));
    args.push("--data-raw", quote(request.body));
  }
  return args.join(" ");
}
