import { execFileSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const tauriBin = resolve(ROOT, "node_modules/.bin/tauri");
const source = resolve(ROOT, "assets/Xaga-iOS-Alpha-1024x1024@1x.png");
const outputDir = resolve(ROOT, "target/alpha-icons");
const marker = resolve(outputDir, "icon.icns");

export const alphaIcons = [
  "32x32.png",
  "128x128.png",
  "128x128@2x.png",
  "icon.icns",
  "icon.ico",
].map((name) => resolve(outputDir, name));

export function isAlphaIconSetCurrent(sourceMtimeMs, markerMtimeMs) {
  return markerMtimeMs !== undefined && markerMtimeMs >= sourceMtimeMs;
}

export function generateAlphaIcons() {
  const current = isAlphaIconSetCurrent(
    statSync(source).mtimeMs,
    existsSync(marker) ? statSync(marker).mtimeMs : undefined
  );
  if (current) {
    console.log("[alpha-icons] icons up to date; skipping regeneration.");
    return;
  }

  console.log(`[alpha-icons] regenerating icons from ${source}`);
  execFileSync(tauriBin, ["icon", source, "--output", outputDir], {
    cwd: ROOT,
    stdio: "inherit",
  });
}
