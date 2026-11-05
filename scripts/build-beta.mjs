// Builds a local beta artifact identified by the current commit.
// The committed app version remains unchanged; build metadata is supplied to
// Tauri only for this invocation.

import { execFileSync, spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { alphaIcons, generateAlphaIcons } from "./alpha-icons.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const TAURI_CONF = resolve(ROOT, "src-tauri/tauri.conf.json");

export function validateCleanStatus(status) {
  if (status !== "") {
    throw new Error(
      "Workspace has uncommitted changes — commit or remove them before building a beta artifact."
    );
  }
}

export function betaVersion(version, commitHash) {
  if (!/^\d+\.\d+\.\d+(?:-rc\.[1-9]\d*)?$/.test(version)) {
    throw new Error(
      `Beta builds require a committed release or numbered release-candidate version (major.minor.patch or major.minor.patch-rc.N); got ${JSON.stringify(version)}. Prepare one with npm run version:rc or npm run version:release, then commit it.`
    );
  }
  if (!/^[0-9a-f]+$/.test(commitHash)) {
    throw new Error(
      `Git returned an invalid commit hash: ${JSON.stringify(commitHash)}`
    );
  }
  return `${version}+beta.${commitHash}`;
}

export function validateUnpublishedVersion(version, tagExists) {
  if (tagExists) {
    throw new Error(
      `Version ${version} already has release tag v${version}. Beta builds require an untagged version; prepare and commit a new version first.`
    );
  }
}

export function tauriBuildArgs(version, icons = alphaIcons) {
  return [
    "run",
    "tauri",
    "--",
    "build",
    "--config",
    JSON.stringify({ version, bundle: { icon: icons } }),
  ];
}

function run(command, args) {
  const result = spawnSync(command, args, { cwd: ROOT, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function main() {
  const status = execFileSync("git", ["status", "--porcelain=v1"], {
    cwd: ROOT,
    encoding: "utf8",
  });
  validateCleanStatus(status);

  const commitHash = execFileSync("git", ["rev-parse", "--short", "HEAD"], {
    cwd: ROOT,
    encoding: "utf8",
  }).trim();
  const { version } = JSON.parse(readFileSync(TAURI_CONF, "utf8"));
  const buildVersion = betaVersion(version, commitHash);
  const matchingTag = execFileSync("git", ["tag", "--list", `v${version}`], {
    cwd: ROOT,
    encoding: "utf8",
  }).trim();
  validateUnpublishedVersion(version, matchingTag !== "");
  console.log(`Building Xaga ${buildVersion}`);
  generateAlphaIcons();
  run("npm", ["run", "audit:report"]);
  run("npm", tauriBuildArgs(buildVersion));
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
