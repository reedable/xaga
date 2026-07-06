// Bumps the Xaga *app* version (not the npm package version).
//
// Source of truth: src-tauri/tauri.conf.json "version". This script reads that,
// computes the next version from a semver keyword, and writes it back to the
// app config, the root package.json, and all three Rust crates so they stay in
// lockstep.
//
// Usage:
//   npm run version:bump -- patch     0.1.0 -> 0.1.1
//   npm run version:bump -- minor     0.1.0 -> 0.2.0
//   npm run version:bump -- major     0.1.0 -> 1.0.0

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// The app-version files. tauri.conf.json is the source of truth (JSON key);
// package.json carries the same JSON-key shape; the Cargo files carry the
// version in their [package] table.
const TAURI_CONF = resolve(ROOT, "src-tauri/tauri.conf.json");
const PACKAGE_JSON = resolve(ROOT, "package.json");
const CARGO_FILES = [
  resolve(ROOT, "src-tauri/Cargo.toml"),
  resolve(ROOT, "crates/xaga-core/Cargo.toml"),
  resolve(ROOT, "crates/xaga-cli/Cargo.toml"),
];

const BUMP = process.argv[2];
if (!["patch", "minor", "major"].includes(BUMP)) {
  console.error(
    `Usage: npm run version:bump -- <patch|minor|major>\n` +
      `  got: ${BUMP === undefined ? "(nothing)" : JSON.stringify(BUMP)}`
  );
  process.exit(1);
}

// Read the current app version from tauri.conf.json (the source of truth).
const confText = readFileSync(TAURI_CONF, "utf8");
const confMatch = confText.match(/"version"\s*:\s*"(\d+)\.(\d+)\.(\d+)"/);
if (!confMatch) {
  console.error(`Could not find a "version": "x.y.z" in ${TAURI_CONF}`);
  process.exit(1);
}

let [major, minor, patch] = confMatch.slice(1, 4).map(Number);
const current = `${major}.${minor}.${patch}`;

if (BUMP === "major") [major, minor, patch] = [major + 1, 0, 0];
else if (BUMP === "minor") [major, minor, patch] = [major, minor + 1, 0];
else patch += 1;

const next = `${major}.${minor}.${patch}`;

// Rewrite tauri.conf.json — regex replace preserves the file's formatting
// exactly (no JSON reserialization / key reordering).
writeFileSync(
  TAURI_CONF,
  confText.replace(/("version"\s*:\s*")\d+\.\d+\.\d+(")/, `$1${next}$2`)
);

// Rewrite the root package.json version. The regex (no /g flag) replaces only
// the first "version" key — the top-level one — so dependency version ranges
// like "^11.14.0" are never matched.
const pkgText = readFileSync(PACKAGE_JSON, "utf8");
const pkgReplaced = pkgText.replace(
  /("version"\s*:\s*")\d+\.\d+\.\d+(")/,
  `$1${next}$2`
);
if (pkgReplaced === pkgText) {
  console.error(`Could not find a "version": "x.y.z" in ${PACKAGE_JSON}`);
  process.exit(1);
}
writeFileSync(PACKAGE_JSON, pkgReplaced);

// Rewrite each Cargo.toml — replace the version only within the [package]
// table (the `[^\[]*?` stops at the next section header, so dependency
// versions elsewhere in the file are never touched).
for (const file of CARGO_FILES) {
  const text = readFileSync(file, "utf8");
  const replaced = text.replace(
    /(\[package\][^[]*?\bversion\s*=\s*")\d+\.\d+\.\d+(")/,
    `$1${next}$2`
  );
  if (replaced === text) {
    console.error(`Could not find a [package] version in ${file}`);
    process.exit(1);
  }
  writeFileSync(file, replaced);
}

console.log(`Xaga app version: ${current} -> ${next} (${BUMP})`);
console.log("Updated:");
console.log("  src-tauri/tauri.conf.json");
console.log("  package.json");
for (const file of CARGO_FILES) {
  console.log(`  ${file.slice(ROOT.length + 1)}`);
}
console.log(
  "\nNote: Cargo.lock still shows the old version until you run a cargo command\n" +
    "(e.g. `cargo check`), which refreshes the workspace packages."
);
