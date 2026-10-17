// Computes the CSP sha256 source value for the reader iframe's executable
// <script> emitted by buildEntryHtml. With --check, verifies that the bundled
// CSP allows that hash in its script-src directive.

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { createServer } from "vite";

export function configuredCsp(config) {
  const csp = config?.app?.security?.csp;
  if (typeof csp !== "string") {
    throw new Error(
      "src-tauri/tauri.conf.json has no string app.security.csp value"
    );
  }
  return csp;
}

export function scriptSrcContainsHash(csp, hash) {
  const scriptSrc = csp
    .split(";")
    .map((directive) => directive.trim().split(/\s+/))
    .find(([name]) => name.toLowerCase() === "script-src");
  return scriptSrc?.slice(1).includes(hash) ?? false;
}

async function readerScriptHash() {
  const server = await createServer({
    appType: "custom",
    logLevel: "silent",
    server: { middlewareMode: true },
  });

  try {
    const { buildEntryHtml } = await server.ssrLoadModule(
      "/src/text/buildEntryHtml.ts"
    );
    const html = buildEntryHtml(
      "",
      {
        themeMode: "light",
        fontFamily: "system",
        fontSize: 16,
        lineHeight: 1.5,
      },
      []
    );
    const match = html.match(/<script>([\s\S]*?)<\/script>/);
    if (!match) {
      throw new Error(
        "Could not locate the executable <script> in buildEntryHtml output"
      );
    }
    const digest = createHash("sha256")
      .update(match[1], "utf8")
      .digest("base64");
    return `'sha256-${digest}'`;
  } finally {
    await server.close();
  }
}

async function main() {
  const hash = await readerScriptHash();
  if (!process.argv.slice(2).includes("--check")) {
    console.log(hash);
    return;
  }

  const config = JSON.parse(
    await readFile(
      new URL("../src-tauri/tauri.conf.json", import.meta.url),
      "utf8"
    )
  );
  if (!scriptSrcContainsHash(configuredCsp(config), hash)) {
    throw new Error(
      `Reader CSP hash mismatch. Add ${hash} to app.security.csp script-src in src-tauri/tauri.conf.json.`
    );
  }
  console.log(`Reader CSP hash is current: ${hash}`);
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
