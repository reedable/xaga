// Validates a committed release version and creates its local annotated tag.
// It never commits or pushes.

import { execFileSync, spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const RELEASE_DOC = "docs/development/release_cycle.md#tagging-a-release";

export function releaseTagName(version) {
  if (!/^\d+\.\d+\.\d+(?:-rc\.[1-9]\d*)?$/.test(version)) {
    throw new Error(
      `Release version must be final or a numbered release candidate; got ${JSON.stringify(version)}. See ${RELEASE_DOC}.`
    );
  }
  return `v${version}`;
}

export function validateCleanStatus(status) {
  if (status !== "") {
    throw new Error(
      `Working tree must be clean before tagging a release. See ${RELEASE_DOC}.`
    );
  }
}

export function validateVersionRecords(records) {
  const expected = records[0]?.version;
  if (!expected) throw new Error("No application versions were found.");
  const mismatches = records.filter(({ version }) => version !== expected);
  if (mismatches.length > 0) {
    const details = records
      .map(({ label, version }) => `${label}=${version}`)
      .join(", ");
    throw new Error(
      `Application versions are not synchronized: ${details}. See ${RELEASE_DOC}.`
    );
  }
  return expected;
}

function jsonVersion(path) {
  return JSON.parse(readFileSync(resolve(ROOT, path), "utf8")).version;
}

function cargoManifestVersion(path) {
  const text = readFileSync(resolve(ROOT, path), "utf8");
  const match = text.match(/\[package\][^[]*?\bversion\s*=\s*"([^"]+)"/);
  if (!match) throw new Error(`Could not read [package] version from ${path}.`);
  return match[1];
}

function cargoLockVersion(name) {
  const text = readFileSync(resolve(ROOT, "Cargo.lock"), "utf8");
  const blocks =
    text.match(/\[\[package\]\][\s\S]*?(?=\n\[\[package\]\]|$)/g) ?? [];
  for (const block of blocks) {
    const packageName = block.match(/^name = "([^"]+)"$/m)?.[1];
    if (packageName === name) {
      const version = block.match(/^version = "([^"]+)"$/m)?.[1];
      if (version) return version;
    }
  }
  throw new Error(`Could not read ${name} version from Cargo.lock.`);
}

function versionRecords() {
  return [
    {
      label: "src-tauri/tauri.conf.json",
      version: jsonVersion("src-tauri/tauri.conf.json"),
    },
    { label: "package.json", version: jsonVersion("package.json") },
    {
      label: "src-tauri/Cargo.toml",
      version: cargoManifestVersion("src-tauri/Cargo.toml"),
    },
    {
      label: "crates/xaga-core/Cargo.toml",
      version: cargoManifestVersion("crates/xaga-core/Cargo.toml"),
    },
    {
      label: "crates/xaga-cli/Cargo.toml",
      version: cargoManifestVersion("crates/xaga-cli/Cargo.toml"),
    },
    { label: "Cargo.lock:xaga", version: cargoLockVersion("xaga") },
    { label: "Cargo.lock:xaga-core", version: cargoLockVersion("xaga-core") },
    { label: "Cargo.lock:xaga-cli", version: cargoLockVersion("xaga-cli") },
  ];
}

function main() {
  const status = execFileSync("git", ["status", "--porcelain=v1"], {
    cwd: ROOT,
    encoding: "utf8",
  });
  validateCleanStatus(status);

  const version = validateVersionRecords(versionRecords());
  const tag = releaseTagName(version);
  const existing = spawnSync(
    "git",
    ["rev-parse", "--verify", "--quiet", `refs/tags/${tag}`],
    { cwd: ROOT, stdio: "ignore" }
  );
  if (existing.error) throw existing.error;
  if (existing.status === 0) {
    throw new Error(`Tag ${tag} already exists. See ${RELEASE_DOC}.`);
  }
  if (existing.status !== 1) {
    throw new Error(`Could not determine whether tag ${tag} already exists.`);
  }

  execFileSync("git", ["tag", "-a", tag, "-m", `Xaga ${tag}`], {
    cwd: ROOT,
    stdio: "inherit",
  });
  const branch = execFileSync("git", ["branch", "--show-current"], {
    cwd: ROOT,
    encoding: "utf8",
  }).trim();
  console.log(`Created local release tag ${tag}.`);
  console.log(
    `Push it with: git push github ${branch || "<branch>"} --follow-tags`
  );
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
