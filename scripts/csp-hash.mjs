// Computes the CSP sha256 source value for the reader iframe's executable
// <script> emitted by buildEntryHtml. The bundled CSP enforces
// `script-src 'self'` (no 'unsafe-inline'), so that inline script only runs if
// its exact hash is allow-listed.
//
// Run this whenever the executable <script> in src/text/buildEntryHtml.ts
// changes, and paste the printed value into app.security.csp (script-src) in
// src-tauri/tauri.conf.json.
//
//   node scripts/csp-hash.mjs
//
// Uses Vite's module loader so the project's extensionless TS imports resolve
// the same way they do in the app build.

import { createHash } from "node:crypto";
import { createServer } from "vite";

const server = await createServer({
  appType: "custom",
  logLevel: "silent",
  server: { middlewareMode: true },
});

try {
  const { buildEntryHtml } = await server.ssrLoadModule(
    "/src/text/buildEntryHtml.ts"
  );

  // The script body is independent of style and annotations; any valid style and
  // empty annotations produce the same executable <script> text.
  const html = buildEntryHtml(
    "",
    { themeMode: "light", fontFamily: "system", fontSize: 16, lineHeight: 1.5 },
    []
  );

  // The executable script is the only <script> with no attributes; the
  // annotations data island is <script type="application/json" ...>.
  const match = html.match(/<script>([\s\S]*?)<\/script>/);
  if (!match) {
    console.error("Could not locate the executable <script> in buildEntryHtml output.");
    process.exit(1);
  }

  const hash = createHash("sha256").update(match[1], "utf8").digest("base64");
  console.log(`'sha256-${hash}'`);
} finally {
  await server.close();
}
