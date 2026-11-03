package dev.harpoon.fixture;

import jakarta.validation.Valid;
import jakarta.validation.constraints.NotBlank;
import org.springframework.http.ResponseEntity;
import org.springframework.security.web.csrf.CsrfToken;
import org.springframework.web.bind.MethodArgumentNotValidException;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.multipart.MultipartFile;
import org.springframework.graphql.data.method.annotation.Argument;
import org.springframework.graphql.data.method.annotation.QueryMapping;
import org.springframework.stereotype.Controller;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;

@RestController
class Endpoints {
    record UserInput(@NotBlank String name) {}
    record ApiUser(long id, String name) {}
    @GetMapping("/api/health") Map<String, Object> health() { return Map.of("status", "UP", "framework", "Spring Boot"); }
    @GetMapping("/api/users") Map<String, Object> users(@RequestParam(defaultValue="0") int page, @RequestParam(defaultValue="10") int size) {
        return Map.of("content", List.of(new ApiUser(1, "Ada")), "page", page, "size", size, "totalElements", 1);
    }
    @PostMapping("/api/users") ResponseEntity<ApiUser> create(@Valid @RequestBody UserInput input) { return ResponseEntity.created(URI.create("/api/users/2")).body(new ApiUser(2, input.name())); }
    @GetMapping("/api/private") Map<String, Object> secret() { return Map.of("authenticated", true); }
    @GetMapping("/api/admin") Map<String, Object> admin() { return Map.of("admin", true); }
    @PostMapping("/api/upload") Map<String, Object> upload(@RequestParam MultipartFile file) throws Exception { return Map.of("filename", file.getOriginalFilename(), "size", file.getSize(), "content", new String(file.getBytes(), StandardCharsets.UTF_8)); }
    @GetMapping("/api/slow") Map<String, Object> slow(@RequestParam(defaultValue="2000") long millis) throws InterruptedException { Thread.sleep(Math.min(millis, 10000)); return Map.of("completed", true); }
    @GetMapping("/session/csrf") Map<String, String> csrf(CsrfToken token) { return Map.of("token", token.getToken(), "parameterName", token.getParameterName(), "headerName", token.getHeaderName()); }
    @GetMapping("/session/private") Map<String, Object> session() { return Map.of("authenticated", true); }
    @ExceptionHandler(MethodArgumentNotValidException.class) ResponseEntity<?> validation(MethodArgumentNotValidException error) {
        return ResponseEntity.badRequest().body(Map.of("status", 400, "title", "Validation failed", "errors", error.getBindingResult().getFieldErrors().stream().map(field -> Map.of("field", field.getField(), "message", field.getDefaultMessage())).toList()));
    }
}

@Controller
class Graphql {
    @QueryMapping Endpoints.ApiUser user(@Argument Long id) { return id == 1 ? new Endpoints.ApiUser(1, "Ada") : null; }
    @QueryMapping String greeting(@Argument String name) { return "Hola, " + name; }
}
