// Prepare a development-version PR without changing main or creating release refs.
import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { nextVersion } from "./bump-version.mjs";
import { readVersionRecords, validateVersionRecords } from "./release-tag.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
function run(command, args, input) {
  return execFileSync(command, args, {
    cwd: root,
    encoding: "utf8",
    input,
  }).trim();
}
function git(...args) {
  return run("git", args);
}
function commitBump(repository, branchName, start, title) {
  const additions = git("diff", "--cached", "--name-only")
    .split("\n")
    .map((path) => ({
      path,
      contents: execFileSync("git", ["show", `:${path}`], {
        cwd: root,
      }).toString("base64"),
    }));
  run(
    "gh",
    ["api", "graphql", "--method", "POST", "--input", "-"],
    JSON.stringify({
      query: `mutation ($input: CreateCommitOnBranchInput!) {
      createCommitOnBranch(input: $input) { commit { oid } }
    }`,
      variables: {
        input: {
          branch: { repositoryNameWithOwner: repository, branchName },
          expectedHeadOid: start,
          message: { headline: title },
          fileChanges: { additions },
        },
      },
    })
  );
}
function main() {
  const [bump, releaseVersion] = process.argv.slice(2);
  if (!["major", "minor", "patch"].includes(bump))
    throw new Error("Expected major, minor, or patch bump");
  if (process.env.GITHUB_REF !== "refs/heads/main")
    throw new Error("Run version bumps on main");
  if (git("status", "--porcelain=v1", "--untracked-files=all"))
    throw new Error("Version bump requires a clean checkout");
  const main = git("rev-parse", "HEAD");
  if (git("ls-remote", "origin", "refs/heads/main").split(/\s+/)[0] !== main)
    throw new Error("main moved; start a fresh run on main");
  const current = validateVersionRecords(readVersionRecords());
  if (
    releaseVersion &&
    current.replace(/-(?:dev|rc\.[0-9]+)$/, "") !== releaseVersion
  ) {
    console.log(
      `Skipping the patch bump for release ${releaseVersion} because main is at ${current}.`
    );
    return;
  }
  const next = nextVersion(current, bump);
  const repository = process.env.GITHUB_REPOSITORY;
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository || "") || !process.env.GH_TOKEN)
    throw new Error(
      "Version bumps require GITHUB_REPOSITORY and the GitHub App token"
    );
  const branch = `version/${next}`;
  const ref = `refs/heads/${branch}`;
  const title = `Start ${next} development`;
  const listPRs = () =>
    JSON.parse(
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
  if (releaseVersion) {
    const existingPR = listPRs()[0]?.url;
    if (existingPR) {
      console.log(`Version bump PR: ${existingPR}`);
      return;
    }
  }

  const existing = git("ls-remote", "origin", ref);
  if (existing) git("fetch", "--no-tags", "origin", ref);
  const head = existing ? git("rev-parse", "FETCH_HEAD") : "";
  // A run interrupted before committing can resume after main advances.
  const preparedSource = head && git("merge-base", head, main) === head;
  const start = preparedSource
    ? head
    : releaseVersion && head
      ? git("rev-parse", `${head}^`)
      : main;
  if (start !== main) git("merge-base", "--is-ancestor", start, main);
  git("switch", "--detach", start);
  if (
    preparedSource &&
    validateVersionRecords(readVersionRecords()) !== current
  )
    throw new Error(
      "Version branch already exists with a different source version"
    );
  run("npm", ["run", `version:${bump}`]);
  if (validateVersionRecords(readVersionRecords()) !== next)
    throw new Error(`Version bump did not produce ${next}`);
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
  if (head && !preparedSource) {
    // An exact bump can be reused even if it predates commit signing.
    if (
      git("show", "-s", "--format=%P%n%T", head) !==
      `${start}\n${git("write-tree")}`
    )
      throw new Error(
        "Version branch already exists with different source or changes"
      );
  } else {
    if (!head)
      run("gh", [
        "api",
        "--method",
        "POST",
        `repos/${repository}/git/refs`,
        "-f",
        `ref=${ref}`,
        "-f",
        `sha=${start}`,
      ]);
    commitBump(repository, branch, start, title);
  }
  const prs = listPRs();
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
  let summary = `Version bump PR: ${url}\n`;
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
