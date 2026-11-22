// Push the verified source and finish its draft. Retrying uses the same package.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { appendFileSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { releaseTagName } from "./release-tag.mjs";
import { reportReleaseStatus } from "./release-status.mjs";

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
function openNextDevelopmentPR(version) {
  const main = "refs/heads/main";
  git("fetch", "--no-tags", "origin", main);
  const temporary = mkdtempSync(join(tmpdir(), "xaga-release-main-"));
  const directory = join(temporary, "main");
  try {
    git("worktree", "add", "--detach", directory, "FETCH_HEAD");
    try {
      const summary = execFileSync(
        process.execPath,
        ["scripts/prepare-version-bump.mjs", "patch", version],
        {
          cwd: directory,
          env: { ...process.env, GITHUB_REF: main },
          encoding: "utf8",
        }
      );
      console.log(summary.trim());
    } finally {
      git("worktree", "remove", "--force", directory);
    }
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
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
  const tagRef = `refs/tags/${state.tag}`;
  if (state.mode !== "tag") {
    if (
      state.branch !==
      `refs/heads/release/${state.version.replace(/-rc\.[0-9]+$/, "")}`
    )
      throw new Error("Invalid branch");
    git("check-ref-format", state.branch);
    const expectedBranch =
      state.mode === "rc" && state.sourceRef === "refs/heads/main"
        ? ""
        : state.start;
    if (state.expectedBranch !== expectedBranch)
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
  reportReleaseStatus(state, "pending");
  report(
    `Remote ${state.tag} exists at ${state.commit}. If draft upload fails, re-run only the failed publication job; do not start another preparation run.`
  );
  let bumpError;
  if (state.mode === "rc" && state.expectedBranch === "") {
    try {
      openNextDevelopmentPR(state.version.replace(/-rc\.[0-9]+$/, ""));
    } catch (error) {
      bumpError = new Error(
        `Could not open the next development version PR. Rerun publication to retry. ${error.message}`
      );
    }
  }
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
  if (state.mode !== "tag") {
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
        `Built release: ${state.tag}. Merge into main through the normal pull request process.`,
      ]);
    report(
      `Release PR targets main from ${branch}. Merging remains a separate action subject to branch protection.`
    );
  }
  report(
    `Draft ${state.tag} contains the verified artifacts. Inspect it before publishing.`
  );
  if (bumpError) throw bumpError;
  reportReleaseStatus(state, "success");
}
try {
  main();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
