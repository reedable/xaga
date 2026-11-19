// Prepare a development-version PR without changing main or creating release refs.
import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { nextVersion } from "./bump-version.mjs";
import { readVersionRecords, validateVersionRecords } from "./release-tag.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
function run(command, args) {
  return execFileSync(command, args, { cwd: root, encoding: "utf8" }).trim();
}
function git(...args) {
  return run("git", args);
}
function main() {
  const bump = process.argv[2];
  if (!["major", "minor", "patch"].includes(bump))
    throw new Error("Expected major, minor, or patch bump");
  if (process.env.GITHUB_REF !== "refs/heads/main")
    throw new Error("Run version bumps on main");
  if (git("status", "--porcelain=v1", "--untracked-files=all"))
    throw new Error("Version bump requires a clean checkout");
  const start = git("rev-parse", "HEAD");
  if (git("ls-remote", "origin", "refs/heads/main").split(/\s+/)[0] !== start)
    throw new Error("main moved; start a fresh run on main");
  const current = validateVersionRecords(readVersionRecords());
  const next = nextVersion(current, bump);
  const branch = `version/${next}`;
  const ref = `refs/heads/${branch}`;
  const title = `Start ${next} development`;

  git("switch", "--detach", start);
  run("npm", ["run", `version:${bump}`]);
  validateVersionRecords(readVersionRecords());
  git(
    "add",
    "--",
    "package.json",
    "Cargo.lock",
    "src-tauri/tauri.conf.json",
    "src-tauri/Cargo.toml",
    "crates/xaga-core/Cargo.toml",
    "crates/xaga-cli/Cargo.toml"
  );
  git("config", "user.name", "github-actions[bot]");
  git(
    "config",
    "user.email",
    "41898282+github-actions[bot]@users.noreply.github.com"
  );
  git("commit", "-m", title);

  const existing = git("ls-remote", "origin", ref);
  if (existing) {
    // Recover PR creation after a successful push, without replacing branch work.
    git("fetch", "--no-tags", "origin", ref);
    if (
      git("rev-parse", "FETCH_HEAD^", "FETCH_HEAD^{tree}") !==
      `${start}\n${git("rev-parse", "HEAD^{tree}")}`
    )
      throw new Error(
        "Version branch already exists with different source or changes"
      );
  } else {
    git("push", `--force-with-lease=${ref}:`, "origin", `HEAD:${ref}`);
  }
  const prs = JSON.parse(
    run("gh", [
      "pr",
      "list",
      "--state",
      "open",
      "--base",
      "main",
      "--head",
      branch,
      "--json",
      "url",
    ])
  );
  const body = `Advance the development version from ${current} to ${next} using npm run version:${bump}.\n\nReview and merge after the required checks pass.`;
  const url =
    prs[0]?.url ||
    run("gh", [
      "pr",
      "create",
      "--base",
      "main",
      "--head",
      branch,
      "--title",
      title,
      "--body",
      body,
    ]);
  let summary = `Version bump PR: ${url}\n\nApprove workflow runs on the PR if GitHub requests approval. If checks do not start, close and reopen the PR manually.\n`;
  if (/^\d+\.\d+\.0-dev$/.test(next))
    summary +=
      "\nReview docs/maintenance/index.md and select recurring maintenance work for the new development cycle.\n";
  console.log(summary);
  if (process.env.GITHUB_STEP_SUMMARY)
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary);
}
try {
  main();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
