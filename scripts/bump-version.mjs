// Advances the Xaga app version through its development and release lifecycle.
// patch/minor/major start development; rc advances candidates; release finalizes.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const TAURI_CONF = resolve(ROOT, "src-tauri/tauri.conf.json");
const PACKAGE_JSON = resolve(ROOT, "package.json");
const CARGO_LOCK = resolve(ROOT, "Cargo.lock");
const CARGO_FILES = [
  resolve(ROOT, "src-tauri/Cargo.toml"),
  resolve(ROOT, "crates/xaga-core/Cargo.toml"),
  resolve(ROOT, "crates/xaga-cli/Cargo.toml"),
];

const VERSION_PATTERN = /^(\d+)\.(\d+)\.(\d+)(?:-(dev|rc\.(\d+)))?$/;
const VERSION_IN_FILE = "\\d+\\.\\d+\\.\\d+(?:-(?:dev|rc\\.\\d+))?";
const COMMANDS = ["patch", "minor", "major", "rc", "release"];
const WORKSPACE_PACKAGES = ["xaga", "xaga-core", "xaga-cli"];

export function nextVersion(current, command) {
  const match = VERSION_PATTERN.exec(current);
  if (!match) {
    throw new Error(
      `Expected version major.minor.patch, major.minor.patch-dev, or major.minor.patch-rc.N; got ${JSON.stringify(current)}`
    );
  }
  if (!COMMANDS.includes(command)) {
    throw new Error(
      `Expected one of ${COMMANDS.join(", ")}; got ${command === undefined ? "nothing" : JSON.stringify(command)}`
    );
  }

  let [, major, minor, patch, stage, rcNumber] = match;
  let parts = [major, minor, patch].map(Number);

  if (["major", "minor", "patch"].includes(command)) {
    if (stage?.startsWith("rc.")) {
      throw new Error(`Cannot ${command}-bump release candidate ${current}`);
    }
    if (command === "major") parts = [parts[0] + 1, 0, 0];
    else if (command === "minor") parts = [parts[0], parts[1] + 1, 0];
    else parts[2] += 1;
    return `${parts.join(".")}-dev`;
  }

  const base = parts.join(".");
  if (command === "rc") {
    if (stage === "dev") return `${base}-rc.1`;
    if (stage?.startsWith("rc.")) {
      return `${base}-rc.${Number(rcNumber) + 1}`;
    }
    throw new Error(
      `Cannot create a release candidate from final version ${current}`
    );
  }

  if (stage === "dev" || stage?.startsWith("rc.")) return base;
  throw new Error(`Version ${current} is already a final release`);
}

function replaceJsonVersion(text, next, file) {
  const replaced = text.replace(
    new RegExp(`("version"\\s*:\\s*")${VERSION_IN_FILE}(")`),
    `$1${next}$2`
  );
  if (replaced === text)
    throw new Error(`Could not find an app version in ${file}`);
  return replaced;
}

function replaceCargoVersion(text, next, file) {
  const replaced = text.replace(
    new RegExp(
      `(\\[package\\][^[]*?\\bversion\\s*=\\s*")${VERSION_IN_FILE}(")`
    ),
    `$1${next}$2`
  );
  if (replaced === text) {
    throw new Error(`Could not find a [package] version in ${file}`);
  }
  return replaced;
}

export function replaceCargoLockVersions(text, next) {
  let replacements = 0;
  const names = WORKSPACE_PACKAGES.join("|");
  const replaced = text.replace(
    new RegExp(
      `(\\[\\[package\\]\\]\\nname = "(?:${names})"\\nversion = ")${VERSION_IN_FILE}(")`,
      "g"
    ),
    (_match, before, after) => {
      replacements += 1;
      return `${before}${next}${after}`;
    }
  );
  if (replacements !== WORKSPACE_PACKAGES.length) {
    throw new Error(
      `Expected ${WORKSPACE_PACKAGES.length} Xaga workspace packages in Cargo.lock; updated ${replacements}`
    );
  }
  return replaced;
}

function main() {
  const command = process.argv[2];
  const confText = readFileSync(TAURI_CONF, "utf8");
  const current = JSON.parse(confText).version;
  const next = nextVersion(current, command);

  writeFileSync(TAURI_CONF, replaceJsonVersion(confText, next, TAURI_CONF));
  const packageText = readFileSync(PACKAGE_JSON, "utf8");
  writeFileSync(
    PACKAGE_JSON,
    replaceJsonVersion(packageText, next, PACKAGE_JSON)
  );

  for (const file of CARGO_FILES) {
    const text = readFileSync(file, "utf8");
    writeFileSync(file, replaceCargoVersion(text, next, file));
  }

  const cargoLockText = readFileSync(CARGO_LOCK, "utf8");
  writeFileSync(CARGO_LOCK, replaceCargoLockVersions(cargoLockText, next));

  console.log(`Xaga app version: ${current} -> ${next} (${command})`);
  console.log("Updated:");
  console.log("  src-tauri/tauri.conf.json");
  console.log("  package.json");
  for (const file of CARGO_FILES)
    console.log(`  ${file.slice(ROOT.length + 1)}`);
  console.log("  Cargo.lock");
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
