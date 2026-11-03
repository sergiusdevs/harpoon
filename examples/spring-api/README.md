# Backend Spring Boot para probar Harpoon

API real con Spring Boot 4.0.0, Spring MVC, Spring Security, Jakarta Validation y
Spring GraphQL. Java 17 o superior y Maven 3.6.3 o superior para compilar.
Solo escucha en `127.0.0.1`.

```sh
mvn -f examples/spring-api/pom.xml package
java -jar examples/spring-api/target/spring-api-0.1.0.jar
```

El puerto predeterminado es `8790`; se puede cambiar con `PORT`. Abre en Harpoon
la colección `examples/spring-api/collection`. Incluye tablas documentadas,
seguridad Basic, validación, GraphQL con esquema SDL y una petición lenta para
probar Cancelar. Las credenciales ficticias son `demo` / `secret`.

Para la batería automática:

```sh
cargo build -p harpoon
npm --prefix apps/desktop run test:spring
```

La batería inicia otro proceso Java en un puerto libre y crea colecciones
temporales. Si ya tienes un servidor, puedes usar `SPRING_API_URL`.
Verifica JSON, paginación, creación 201, validación 400, Basic 200/401/403,
CSRF, cookies de sesión entre procesos, archivos multipart y consultas/errores
con un motor GraphQL real. `/api/**` es una API de prueba sin sesión;
`/session/**` mantiene CSRF habilitado y comprueba el login con token y cookie.

Referencias: [Spring Boot](https://docs.spring.io/spring-boot/4.0/system-requirements.html),
[CSRF en Spring Security](https://docs.spring.io/spring-security/reference/servlet/exploits/csrf.html).
