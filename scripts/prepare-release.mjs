// Shared GitHub release preparation. State and packages live outside the checkout.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  appendFileSync,
  copyFileSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { nextVersion } from "./bump-version.mjs";
import { reportReleaseStatus } from "./release-status.mjs";
import {
  releaseTagName,
  readVersionRecords,
  validateVersionRecords,
} from "./release-tag.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const versionFiles = [
  "package.json",
  "Cargo.lock",
  "src-tauri/tauri.conf.json",
  "src-tauri/Cargo.toml",
  "crates/xaga-core/Cargo.toml",
  "crates/xaga-cli/Cargo.toml",
];
const target = "aarch64-apple-darwin";
function run(command, args) {
  return execFileSync(command, args, { cwd: root, encoding: "utf8" }).trim();
}
function git(...args) {
  return run("git", args);
}
function read(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}
function write(path, value) {
  writeFileSync(path, JSON.stringify(value, null, 2) + "\n");
}
function outside(name) {
  const path = resolve(process.env[name] || "");
  if (path === root || path.startsWith(root + "/"))
    throw new Error(`${name} must be outside the checkout`);
  return path;
}
function validatePreparedSource(state) {
  if (
    git("rev-parse", "HEAD") !== state.commit ||
    git("write-tree") !== state.tree ||
    git("ls-files", "--others", "--exclude-standard")
  ) {
    throw new Error(
      "Source changed after release preparation; refusing to commit or publish the build"
    );
  }
  git("diff", "--exit-code");
}
function prepare(mode, statePath) {
  if (!["rc", "release"].includes(mode))
    throw new Error("Expected rc or release mode");
  if (git("status", "--porcelain=v1", "--untracked-files=all"))
    throw new Error("Release preparation requires a clean checkout");
  const start = git("rev-parse", "HEAD");
  const ref = process.env.GITHUB_REF || "";
  const current = validateVersionRecords(readVersionRecords());
  const nextPatch =
    mode === "rc" &&
    ref === "refs/heads/main" &&
    /^\d+\.\d+\.\d+$/.test(current);
  const versionForBranch = nextPatch
    ? nextVersion(current, "rc-next-patch")
    : current;
  const base = versionForBranch.replace(/-(?:dev|rc\.[0-9]+)$/, "");
  const branch = `refs/heads/release/${base}`;
  let expectedBranch = start;
  const first = mode === "rc" && (current.endsWith("-dev") || nextPatch);
  const candidate = /-rc\.[1-9][0-9]*$/.test(current);
  const restoreBranch = mode === "rc" && ref === "refs/heads/main" && candidate;
  if (first || restoreBranch) {
    if (ref !== "refs/heads/main")
      throw new Error("Start an RC from main with a development version");
    if (git("ls-remote", "origin", branch)) {
      if (restoreBranch)
        throw new Error(
          `The RC is in an inconsistent state: main has ${current} and release/${base} already exists. This RC must be abandoned. See docs/development/release_cycle.md#abandon-an-inconsistent-rc.`
        );
      throw new Error(`Release branch already exists; select release/${base}`);
    }
    expectedBranch = "";
  } else {
    if (ref !== branch || !candidate)
      throw new Error(
        "Select the matching release/x.y.z branch with an RC version"
      );
    if (git("ls-remote", "origin", branch).split(/\s+/)[0] !== start)
      throw new Error("Release branch moved; start a fresh run");
  }
  if (nextPatch)
    run(process.execPath, ["scripts/bump-version.mjs", "rc-next-patch"]);
  else run("npm", ["run", `version:${mode}`]);
  const version = validateVersionRecords(readVersionRecords());
  const tag = releaseTagName(version);
  const changed = git("diff", "--name-only").split("\n");
  if (
    changed.length !== versionFiles.length ||
    changed.some((path) => !versionFiles.includes(path))
  )
    throw new Error("Unexpected version preparation changes");
  git("add", "--", ...versionFiles);
  git("config", "user.name", "github-actions[bot]");
  git(
    "config",
    "user.email",
    "41898282+github-actions[bot]@users.noreply.github.com"
  );
  git("commit", "-m", `Prepare ${tag}`);
  const identity = {
    mode,
    start,
    commit: git("rev-parse", "HEAD"),
    branch,
    expectedBranch,
    sourceRef: ref,
    runId: process.env.GITHUB_RUN_ID,
    runAttempt: process.env.GITHUB_RUN_ATTEMPT,
    version,
    tag,
    tree: git("write-tree"),
  };
  write(statePath, identity);
  reportReleaseStatus(identity, "pending");
}
function seal(statePath, directory) {
  const state = read(statePath);
  validatePreparedSource(state);
  const bundle = join(root, "target", target, "release/bundle");
  run("codesign", [
    "--verify",
    "--deep",
    "--strict",
    "--verbose=2",
    join(bundle, "macos/Xaga.app"),
  ]);
  mkdirSync(directory, { recursive: true });
  const archive = `Xaga_${state.version}_aarch64.app.tar.gz`;
  run("tar", [
    "-czf",
    join(directory, archive),
    "-C",
    join(bundle, "macos"),
    "Xaga.app",
  ]);
  const dmgs = readdirSync(join(bundle, "dmg")).filter((name) =>
    name.endsWith(".dmg")
  );
  if (dmgs.length !== 1)
    throw new Error("Expected exactly one macOS disk image");
  copyFileSync(join(bundle, "dmg", dmgs[0]), join(directory, dmgs[0]));
  validatePreparedSource(state);
  // The existing tag command rechecks clean status and every version record.
  run("npm", ["run", "release:tag"]);
  git(
    "bundle",
    "create",
    join(directory, "release.bundle"),
    `refs/tags/${state.tag}`,
    `^${state.start}`
  );
  const commit = git("rev-parse", "HEAD");
  if (git("rev-parse", `${commit}^{tree}`) !== state.tree)
    throw new Error("Release commit does not match the built source tree");
  const assets = [archive, ...dmgs].map((name) => ({
    name,
    sha256: createHash("sha256")
      .update(readFileSync(join(directory, name)))
      .digest("hex"),
  }));
  write(join(directory, "release.json"), {
    ...state,
    commit,
    tagObject: git("rev-parse", `refs/tags/${state.tag}`),
    assets,
  });
  if (process.env.GITHUB_STEP_SUMMARY)
    appendFileSync(
      process.env.GITHUB_STEP_SUMMARY,
      `Verified ${state.tag} at ${commit}. Artifacts retained for 14 days.\n`
    );
}
try {
  const [command, mode] = process.argv.slice(2);
  const statePath = outside("RELEASE_STATE");
  if (command === "prepare") prepare(mode, statePath);
  else if (command === "seal") seal(statePath, outside("RELEASE_PACKAGE"));
  else throw new Error("Expected prepare or seal");
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
