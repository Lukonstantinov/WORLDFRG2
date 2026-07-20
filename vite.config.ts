import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

const r = (p: string) => fileURLToPath(new URL(p, import.meta.url));

// Path aliases — mirror of tsconfig.json "paths". Exact (`@bridge`, `@types`,
// `@goods`) aliases use anchored regexes so they don't swallow longer specifiers;
// prefix aliases (`@state`, `@canvas`, `@ui`, `@app`) map a folder.
export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  resolve: {
    alias: [
      { find: /^@bridge$/, replacement: r("./src/bridge/index.ts") },
      { find: /^@types$/, replacement: r("./src/types/index.ts") },
      { find: /^@goods$/, replacement: r("./src/goods.ts") },
      { find: "@state", replacement: r("./src/state") },
      { find: "@canvas", replacement: r("./src/canvas") },
      { find: "@ui", replacement: r("./src/ui") },
      { find: "@app", replacement: r("./src") },
    ],
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
