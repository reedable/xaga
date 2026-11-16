// Push the verified source and finish its draft. Retrying uses the same package.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { appendFileSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { releaseTagName } from "./release-tag.mjs";

import { validateMainProtection } from "./release-evidence.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
function run(command, args) {
  return execFileSync(command, args, { cwd: root, encoding: "utf8" }).trim();
}
function git(...args) {
  return run("git", args);
}
function report(message) {
  console.log(message);
  if (process.env.GITHUB_STEP_SUMMARY)
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, message + "\n");
}
function main() {
  const directory = resolve(process.env.RELEASE_PACKAGE);
  const state = JSON.parse(
    readFileSync(join(directory, "release.json"), "utf8")
  );
  if (
    releaseTagName(state.version) !== state.tag ||
    !["rc", "release", "tag"].includes(state.mode)
  )
    throw new Error("Invalid release identity");
  for (const value of [state.start, state.commit, state.tree, state.tagObject])
    if (!/^[0-9a-f]{40}$/.test(value))
      throw new Error("Invalid Git object identity");
  if (!Array.isArray(state.assets) || state.assets.length !== 2)
    throw new Error("Expected app archive and disk image");
  for (const asset of state.assets) {
    if (!/^[A-Za-z0-9_.-]+$/.test(asset.name))
      throw new Error("Invalid asset name");
    const hash = createHash("sha256")
      .update(readFileSync(join(directory, asset.name)))
      .digest("hex");
    if (hash !== asset.sha256)
      throw new Error(`Artifact checksum mismatch: ${asset.name}`);
  }
  if (state.mode === "release") validateMainProtection(root);
  const tagRef = `refs/tags/${state.tag}`;
  if (state.mode !== "tag") {
    if (
      state.branch !==
      `refs/heads/release/${state.version.replace(/-rc\.[0-9]+$/, "")}`
    )
      throw new Error("Invalid branch");
    git("check-ref-format", state.branch);
    if (
      state.expectedBranch !== state.start &&
      !(
        state.mode === "rc" &&
        state.sourceRef === "refs/heads/develop" &&
        state.expectedBranch === ""
      )
    )
      throw new Error("Invalid release branch update expectation");
    git("fetch", join(directory, "release.bundle"), tagRef);
    if (
      git("rev-parse", "FETCH_HEAD") !== state.tagObject ||
      git("rev-parse", "FETCH_HEAD^{commit}") !== state.commit ||
      git("rev-parse", `${state.commit}^`) !== state.start
    )
      throw new Error("Prepared commit or tag mismatch");
  }
  if (git("rev-parse", `${state.commit}^{tree}`) !== state.tree)
    throw new Error("Prepared source tree mismatch");
  const remote = new Map(
    git(
      "ls-remote",
      "origin",
      tagRef,
      `${tagRef}^{}`,
      ...(state.branch ? [state.branch] : [])
    )
      .split("\n")
      .filter(Boolean)
      .map((line) => {
        const [sha, ref] = line.split(/\s+/);
        return [ref, sha];
      })
  );
  const tagMatches =
    remote.get(tagRef) === state.tagObject &&
    (remote.get(`${tagRef}^{}`) || remote.get(tagRef)) === state.commit;
  if (tagMatches && state.mode !== "tag") {
    // A matching tag alone does not establish that the atomic push succeeded.
    // Allow later branch commits, but never treat a tag-only push as recovery.
    git("fetch", "--no-tags", "origin", state.branch);
    git("merge-base", "--is-ancestor", state.commit, "FETCH_HEAD");
  }
  if (!tagMatches) {
    if (
      state.mode === "tag" ||
      remote.has(tagRef) ||
      (remote.get(state.branch) || "") !== state.expectedBranch
    )
      throw new Error(
        "Branch moved or tag conflicts; no refs updated. Start a fresh run."
      );
    // Exact leases are checked by the server; atomic push prevents partial refs.
    // The commit's sole parent was checked above, so this can only fast-forward.
    if (
      git("rev-list", "--parents", "-n", "1", state.commit) !==
      `${state.commit} ${state.start}`
    )
      throw new Error(
        "Release commit must have exactly the starting commit as its parent"
      );
    git(
      "push",
      "--atomic",
      `--force-with-lease=${state.branch}:${state.expectedBranch}`,
      `--force-with-lease=${tagRef}:`,
      "origin",
      `${state.commit}:${state.branch}`,
      `${state.tagObject}:${tagRef}`
    );
  }
  report(
    `Remote ${state.tag} exists at ${state.commit}. If draft upload fails, re-run only the failed publication job; do not start another preparation run.`
  );
  const repo = process.env.GITHUB_REPOSITORY;
  if (!/^[\w.-]+\/[\w.-]+$/.test(repo || ""))
    throw new Error("GITHUB_REPOSITORY is required");
  // Listing distinguishes a missing draft from an API/authentication failure.
  const pages = JSON.parse(
    run("gh", [
      "api",
      "--paginate",
      "--slurp",
      `repos/${repo}/releases?per_page=100`,
    ])
  );
  const release = pages.flat().find((value) => value.tag_name === state.tag);
  const prerelease = state.version.includes("-rc.");
  if (release && (!release.draft || release.prerelease !== prerelease))
    throw new Error(
      "Existing release is published or has conflicting prerelease status; refusing to alter it"
    );
  if (!release)
    run("gh", [
      "release",
      "create",
      state.tag,
      "--repo",
      repo,
      "--verify-tag",
      "--draft",
      "--generate-notes",
      "--title",
      `Xaga ${state.tag}`,
      ...(prerelease ? ["--prerelease"] : []),
    ]);
  run("gh", [
    "release",
    "upload",
    state.tag,
    ...state.assets.map((asset) => join(directory, asset.name)),
    join(directory, "release.json"),
    "--repo",
    repo,
    "--clobber",
  ]);
  if (state.mode === "release") {
    const branch = state.branch.replace("refs/heads/", "");
    // Never direct a PR at a newer, unverified release-branch head during recovery.
    const head = git("ls-remote", "origin", state.branch).split(/\s+/)[0];
    if (head !== state.commit)
      throw new Error(
        "Release branch advanced; artifacts recovered, but refusing to create a PR for unverified changes"
      );
    const prs = JSON.parse(
      run("gh", [
        "pr",
        "list",
        "--repo",
        repo,
        "--base",
        "main",
        "--head",
        branch,
        "--state",
        "open",
        "--json",
        "number",
      ])
    );
    if (prs.length === 0)
      run("gh", [
        "pr",
        "create",
        "--repo",
        repo,
        "--base",
        "main",
        "--head",
        branch,
        "--title",
        `Release ${state.tag}`,
        "--body",
        `Verified final release: ${state.tag}. The Release PR verification status must pass before merging. Retain this release branch.`,
      ]);
    run("gh", [
      "pr",
      "merge",
      branch,
      "--repo",
      repo,
      "--auto",
      "--merge",
      "--match-head-commit",
      state.commit,
    ]);
    report(
      `Release PR targets main from ${branch}. Main is updated only through its protected merge process.`
    );
  }
  report(
    `Draft ${state.tag} contains the verified artifacts. Inspect it before publishing.`
  );
}
try {
  main();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
