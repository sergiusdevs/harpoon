import test, { before, after } from "node:test";
import assert from "node:assert/strict";
import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
const execute = promisify(execFile);
const binary = process.env.HARPOON_BIN ?? fileURLToPath(new URL("../target/debug/harpoon", import.meta.url));
let address = process.env.SPRING_API_URL;
let server;
const directories = [];
before(async () => {
  if (address) return;
  const jar = fileURLToPath(new URL("../examples/spring-api/target/spring-api-0.1.0.jar", import.meta.url));
  server = spawn("java", ["-jar", jar, "--server.port=0"], { stdio: ["ignore", "pipe", "pipe"] });
  await new Promise((resolve, reject) => {
    let output = "";
    const timer = setTimeout(() => { server.kill(); reject(new Error(`Spring no arrancó: ${output.slice(-3000)}`)); }, 25000);
    const collect = (chunk) => {
      output += chunk;
      const match = output.match(/Tomcat started on port (\d+)/);
      if (match) { address = `http://127.0.0.1:${match[1]}`; clearTimeout(timer); resolve(); }
    };
    server.stdout.on("data", collect); server.stderr.on("data", collect);
    server.once("error", (error) => { clearTimeout(timer); reject(error); });
    server.once("exit", (code) => { clearTimeout(timer); reject(new Error(`Spring salió (${code}): ${output.slice(-3000)}. Compila antes examples/spring-api/pom.xml.`)); });
  });
});
after(async () => { server?.kill(); await Promise.all(directories.map((root) => rm(root, { recursive: true, force: true }))); });
async function collection() {
  const root = await mkdtemp(join(tmpdir(), "harpoon-spring-")); directories.push(root);
  await mkdir(join(root, "requests")); await mkdir(join(root, "environments"));
  await writeFile(join(root, "harpoon.toml"), 'name = "Spring integration"\nformat = 1\ndefault_environment = "dev"\n');
  await writeFile(join(root, "environments/dev.toml"), `[vars]\nbase_url = "${address}"\n`);
  return root;
}
async function request(root, key, path, extra = "", method = "GET", type = "http") {
  await writeFile(join(root, "requests", `${key}.toml`), `name = "${key}"\ntype = "${type}"\nmethod = "${method}"\nurl = "{{base_url}}${path}"\n${extra}\n`);
}
async function send(root, key, status = 200) {
  const { stdout } = await execute(binary, ["-C", root, "send", key, "--json"], { timeout: 10000 });
  const result = JSON.parse(stdout); assert.equal(result.response.status, status); return result;
}
const basic = '[auth]\ntype = "basic"\nusername = "demo"\npassword = "secret"';

test("Spring MVC: JSON y paginación", async () => {
  const root = await collection();
  await request(root, "health", "/api/health"); assert.equal((await send(root, "health")).response.body.framework, "Spring Boot");
  await request(root, "users", "/api/users?page=2&size=5");
  const users = (await send(root, "users")).response.body;
  assert.equal(users.page, 2); assert.equal(users.size, 5); assert.equal(users.content[0].name, "Ada");
});

test("Spring Validation: creación 201 y rechazo de @NotBlank con 400", async () => {
  const root = await collection();
  await request(root, "create", "/api/users", '[body]\ntype = "json"\njson = \'{"name":"Grace"}\'\n[expect]\nstatus = 201', "POST");
  assert.equal((await send(root, "create", 201)).response.body.name, "Grace");
  await request(root, "invalid", "/api/users", '[body]\ntype = "json"\njson = \'{"name":""}\'\n[expect]\nstatus = 400', "POST");
  assert.equal((await send(root, "invalid", 400)).response.body.errors[0].field, "name");
});

test("Spring Security: Basic 200, sin credenciales 401 y rol insuficiente 403", async () => {
  const root = await collection();
  await request(root, "private", "/api/private", basic); await send(root, "private");
  await request(root, "no-auth", "/api/private", '[expect]\nstatus = 401'); await send(root, "no-auth", 401);
  await request(root, "admin", "/api/admin", basic + '\n[expect]\nstatus = 403'); await send(root, "admin", 403);
});

test("Spring Security: cookie, token CSRF y login de sesión reales", async () => {
  const root = await collection();
  await request(root, "csrf", "/session/csrf", '[scripts]\npost = \'harpoon.vars.set("csrf", harpoon.res.json().token);\'');
  await send(root, "csrf");
  await request(root, "login", "/session/login", '[body]\ntype = "form"\n[body.fields]\nusername = "demo"\npassword = "secret"\n_csrf = "{{csrf}}"', "POST");
  assert.equal((await send(root, "login")).response.body.authenticated, true);
  await request(root, "session", "/session/private"); assert.equal((await send(root, "session")).response.body.authenticated, true);
});

test("Spring MVC: subida MultipartFile con nombre y contenido", async () => {
  const root = await collection(); await writeFile(join(root, "sample.txt"), "archivo desde Harpoon");
  await request(root, "upload", "/api/upload", '[body]\ntype = "multipart"\n[[body.parts]]\ntype = "file"\nname = "file"\npath = "sample.txt"', "POST");
  const body = (await send(root, "upload")).response.body; assert.equal(body.filename, "sample.txt"); assert.equal(body.content, "archivo desde Harpoon");
});

test("Spring GraphQL: esquema real, variables, operación y errores HTTP 200", async () => {
  const root = await collection();
  await request(root, "query", "/graphql", '[body]\ntype = "graphql"\nquery = "query FindUser($id: ID!) { user(id: $id) { id name } }"\nvariables = \'{"id":1}\'\noperation_name = "FindUser"', "POST", "graphql");
  assert.equal((await send(root, "query")).response.body.data.user.name, "Ada");
  await request(root, "invalid", "/graphql", '[body]\ntype = "graphql"\nquery = "{ unknownField }"\nvariables = "{}"', "POST", "graphql");
  assert.match((await send(root, "invalid")).response.body.errors[0].message, /unknownField/);
});
