# ADR-001: Arpón es 100 % offline

**Estado:** aceptado · **Fecha:** 2026-11-02

## Contexto

Las herramientas de este tipo tienden a añadir nube, cuentas, telemetría e IA.
Arpón nace para equipos que trabajan dentro de sus propios controles de red,
acceso y almacenamiento, y para quien no quiere depender de un proveedor.

## Decisión

1. El binario **solo** abre conexiones hacia los endpoints que el usuario ha
   definido en sus solicitudes o ha pedido explícitamente ejecutar.
2. No hay telemetría, comprobación de actualizaciones, descarga de recursos
   (fuentes, CDNs, runtimes) ni "inicio de sesión".
3. Todo lo necesario para funcionar va embebido: motor de scripts, plantillas
   de documentación, servidor mock, bundle de CAs (`webpki-roots`) con opción
   de usar el del sistema.
4. La IA y los gestores de secretos externos quedan fuera del núcleo. Si se
   añaden, serán crates opcionales desactivados por defecto, y la IA solo
   contra modelos que corran en la máquina del usuario.
5. Los secretos nunca se escriben en archivos versionados: viven en
   `*.secrets.toml` (ignorados por Git), variables de entorno o el llavero del SO.

## Consecuencias

- Compilamos TLS con `rustls` para no depender de OpenSSL del sistema.
- La colaboración se hace con Git; la app no reimplementa Git, delega en el
  binario del sistema y solo muestra estado.
- Cada funcionalidad nueva debe responder: "¿funciona sin red?". Si no, va
  detrás de un *feature flag* y se documenta aquí.
