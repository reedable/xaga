import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const tauriBin = resolve(ROOT, "node_modules/.bin/tauri");
const source = resolve(ROOT, "assets/Xaga-iOS-Alpha-1024x1024@1x.png");
const outputDir = resolve(ROOT, "target/alpha-icons");

export const alphaIcons = [
  "32x32.png",
  "128x128.png",
  "128x128@2x.png",
  "icon.icns",
  "icon.ico",
].map((name) => resolve(outputDir, name));

export function generateAlphaIcons() {
  execFileSync(tauriBin, ["icon", source, "--output", outputDir], {
    cwd: ROOT,
    stdio: "inherit",
  });
}
