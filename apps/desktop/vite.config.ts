import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import pkg from "./package.json" with { type: "json" };

// Tauri espera un puerto fijo y que Vite no se queje si está ocupado.
export default defineConfig({
  plugins: [svelte()],
  // Versión de la app (única fuente: package.json) para la interfaz.
  define: { __APP_VERSION__: JSON.stringify(pkg.version) },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: ["es2022", "safari16"],
    sourcemap: false,
  },
});
