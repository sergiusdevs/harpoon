export type RequestFailure = { code: string; title: string; message: string; detail: string };
export function requestFailure(cause: unknown): RequestFailure {
  if (cause && typeof cause === "object" && "code" in cause && "title" in cause && "message" in cause) {
    const value = cause as RequestFailure;
    return { code: String(value.code), title: String(value.title), message: String(value.message), detail: String(value.detail ?? "") };
  }
  const detail = cause instanceof Error ? cause.message : String(cause);
  const text = detail.toLowerCase();
  const rule = /variable sin definir|variable sin resolver/.test(text) ? ["variable", "Variable sin resolver", "Define la variable en el entorno activo o en la colección."]
    : /certificate|certificado|tls/.test(text) ? ["tls", "Error de certificado o TLS", "Revisa la CA y los certificados de la colección."]
    : /timeout|timed out|tiempo agotado/.test(text) ? ["timeout", "Tiempo de espera agotado", "Comprueba el servidor o ajusta el timeout de la colección."]
    : /connection refused|failed to fetch|dns|network is unreachable/.test(text) ? ["connection", "No se pudo conectar", "Comprueba el servidor, la URL y el puerto."]
    : ["request", "No se pudo enviar", "Revisa los detalles de la solicitud."];
  return { code: rule[0], title: rule[1], message: rule[2], detail };
}
