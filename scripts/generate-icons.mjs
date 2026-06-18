// Regenerates the app icons in src-tauri/icons from the canonical source image.
//
// The source artwork lives in assets/ (version-controlled, durable), while the
// generated set in src-tauri/icons is treated as a derived build artifact: it
// can be deleted at any time and this script will recreate it from the source.
//
// Wired into `beforeBuildCommand` in src-tauri/tauri.conf.json so `tauri build`
// regenerates icons automatically. Run standalone with:
//
//   node scripts/generate-icons.mjs
//
// Regeneration is skipped when the output is already up to date (icon.icns is
// present and at least as new as the source), so unchanged builds pay nothing.

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = resolve(root, "assets/Xaga-iOS-Default-1024x1024@1x.png");
const outputDir = resolve(root, "src-tauri/icons");
// tauri icon always emits icon.icns; use it as the freshness marker.
const marker = resolve(outputDir, "icon.icns");
const tauriBin = resolve(root, "node_modules/.bin/tauri");

if (!existsSync(source)) {
  console.error(`[generate-icons] source image not found: ${source}`);
  process.exit(1);
}

const upToDate =
  existsSync(marker) && statSync(marker).mtimeMs >= statSync(source).mtimeMs;

if (upToDate) {
  console.log("[generate-icons] icons up to date; skipping regeneration.");
  process.exit(0);
}

mkdirSync(outputDir, { recursive: true });
console.log(`[generate-icons] regenerating icons from ${source}`);
execFileSync(tauriBin, ["icon", source, "-o", outputDir], {
  stdio: "inherit",
  cwd: root,
});
