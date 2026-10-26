// Compiles the canonical macOS Icon Composer project into the asset catalog
// consumed by macOS 26 and newer. The generated Assets.car is committed so
// release builds do not depend on the GitHub runner having a recent Xcode.
//
// Requires full Xcode with Icon Composer support. Run after changing anything
// under assets/Xaga-TopHat.icon:
//
//   npm run icons:macos

import { execFileSync } from "node:child_process";
import { cpSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = resolve(root, "assets/Xaga-TopHat.icon");
const output = resolve(root, "src-tauri/Assets.car");
const legacyOutput = resolve(root, "src-tauri/icons/macos/icon.icns");
const temporaryDirectory = mkdtempSync(join(tmpdir(), "xaga-macos-icon-"));

try {
  execFileSync(
    "xcrun",
    [
      "actool",
      source,
      "--compile",
      temporaryDirectory,
      "--platform",
      "macosx",
      "--minimum-deployment-target",
      "14.0",
      "--app-icon",
      "Xaga-TopHat",
      "--include-all-app-icons",
      "--output-partial-info-plist",
      join(temporaryDirectory, "partial.plist"),
    ],
    { cwd: root, stdio: "inherit" },
  );

  cpSync(join(temporaryDirectory, "Xaga-TopHat.icns"), legacyOutput);
  cpSync(join(temporaryDirectory, "Assets.car"), output);
  console.log(`[generate-macos-icon] wrote ${legacyOutput}`);
  console.log(`[generate-macos-icon] wrote ${output}`);
} finally {
  rmSync(temporaryDirectory, { recursive: true, force: true });
}
