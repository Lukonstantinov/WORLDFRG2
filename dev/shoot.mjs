/** Screenshot the panel schematic harness with the pre-installed Chromium.
 *
 *  The harness is BUILT to static files and loaded over file:// — no dev server to
 *  babysit, and the shot then exercises the same bundling path the app uses.
 *
 *  Usage:  npx vite build --config dev/vite.schematic.ts && node dev/shoot.mjs [out.png]
 */
import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { resolve, extname, join } from "node:path";

const out = process.argv[2] ?? "dev/schematics.png";
const ROOT = resolve("dev/dist-schematic");
// ES modules cannot be loaded over file:// (CORS blocks them), so serve the built
// bundle from a throwaway in-process static server. No dependency, no lifecycle to
// manage across shells — it dies with this script.
const MIME = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css",
               ".svg": "image/svg+xml", ".png": "image/png" };
const server = createServer(async (req, res) => {
  const path = join(ROOT, decodeURIComponent((req.url ?? "/").split("?")[0]));
  try {
    const body = await readFile(path);
    res.writeHead(200, { "content-type": MIME[extname(path)] ?? "application/octet-stream" });
    res.end(body);
  } catch {
    // /favicon.ico is the only expected miss; answer it so the console stays clean
    // and a real 404 still means a real broken asset.
    if (path.endsWith("favicon.ico")) { res.writeHead(204); res.end(); return; }
    res.writeHead(404); res.end("not found");
  }
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const page_url = `http://127.0.0.1:${server.address().port}/schematic.html`;

// The environment pre-installs Chromium at a pinned revision; launch THAT rather
// than letting Playwright hunt for a build matching its own version (and never run
// `playwright install` here — the browser is already on disk).
const browser = await chromium.launch({
  executablePath: process.env.PW_CHROME ?? "/opt/pw-browsers/chromium-1194/chrome-linux/chrome",
});
const page = await browser.newPage({ viewport: { width: 1700, height: 900 }, deviceScaleFactor: 2 });
const errs = [];
page.on("pageerror", (e) => errs.push(String(e)));
page.on("console", (m) => { if (m.type() === "error") errs.push(m.text()); });
await page.goto(page_url, { waitUntil: "load" });
await page.waitForTimeout(1200); // let the stubbed async bridge calls resolve into state
await page.screenshot({ path: out, fullPage: true });
await browser.close();
server.close();
if (errs.length) { console.error("PAGE ERRORS:\n" + errs.join("\n")); process.exitCode = 1; }
else console.log("clean render →", out);
