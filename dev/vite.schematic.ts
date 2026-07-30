/** Vite config for the panel schematic harness (see dev/schematic.tsx). Reuses the
 *  app's aliases so the real components resolve exactly as they do in the app. */
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";
const r = (p: string) => fileURLToPath(new URL(p, import.meta.url));
export default defineConfig({
  root: r("."),
  plugins: [react()],
  resolve: {
    alias: [
      { find: /^@bridge$/, replacement: r("../src/bridge/index.ts") },
      { find: /^@types$/, replacement: r("../src/types/index.ts") },
      { find: /^@goods$/, replacement: r("../src/goods.ts") },
      { find: "@state", replacement: r("../src/state") },
      { find: "@canvas", replacement: r("../src/canvas") },
      { find: "@ui", replacement: r("../src/ui") },
      { find: "@app", replacement: r("../src") },
    ],
  },
  // Built to static files and loaded over file:// by dev/shoot.mjs — no dev server
  // to babysit, and the screenshot then exercises the same bundling the app uses.
  base: "./",
  build: { outDir: r("./dist-schematic"), emptyOutDir: true,
           rollupOptions: { input: r("./schematic.html") } },
});
