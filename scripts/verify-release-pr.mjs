// Runs from the trusted base branch. Inspect candidate Git objects, never execute them.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import {
  readVerifiedRelease,
  validateMainProtection,
} from "./release-evidence.mjs";

const repo = process.env.GITHUB_REPOSITORY;
const run = (command, args) =>
  execFileSync(command, args, { encoding: "utf8" }).trim();
const gh = (...args) => run("gh", args);
const git = (...args) => run("git", args);
function status(sha, state, description) {
  gh(
    "api",
    `repos/${repo}/statuses/${sha}`,
    "-f",
    `state=${state}`,
    "-f",
    "context=Release PR verification",
    "-f",
    `description=${description}`
  );
}
function verify(number) {
  const pr = JSON.parse(gh("api", `repos/${repo}/pulls/${number}`));
  if (pr.state !== "open" || pr.base.ref !== "main") return;
  const head = pr.head.sha;
  status(head, "pending", "Checking verified final source and proposed merge");
  try {
    validateMainProtection(process.cwd());
    if (
      pr.head.repo?.full_name !== repo ||
      !/^release\/\d+\.\d+\.\d+$/.test(pr.head.ref)
    )
      throw new Error(
        "Only a same-repository release branch may merge into main"
      );
    git("fetch", "--no-tags", "origin", `refs/pull/${number}/head`);
    if (git("rev-parse", "FETCH_HEAD") !== head)
      throw new Error("PR changed during verification");
    git("fetch", "--no-tags", "origin", "refs/heads/main");
    const base = git("rev-parse", "FETCH_HEAD");
    const version = JSON.parse(
      git("show", `${head}:src-tauri/tauri.conf.json`)
    ).version;
    if (
      !/^\d+\.\d+\.\d+$/.test(version) ||
      pr.head.ref !== `release/${version}`
    )
      throw new Error("Main accepts verified final versions only");
    const tag = `v${version}`;
    git("fetch", "origin", `refs/tags/${tag}:refs/tags/${tag}`);
    const record = readVerifiedRelease(
      version,
      head,
      `refs/heads/${pr.head.ref}`,
      process.cwd()
    );
    const tree = git("merge-tree", "--write-tree", base, head).split("\n")[0];
    if (tree !== record.tree)
      throw new Error(
        "Proposed merge differs from the verified build; prepare a new candidate"
      );
    const current = JSON.parse(gh("api", `repos/${repo}/pulls/${number}`));
    if (current.head.sha !== head || current.base.sha !== base)
      throw new Error("PR or main changed during verification");
    status(
      head,
      "success",
      "Final build and proposed merge have identical source trees"
    );
  } catch (error) {
    status(
      head,
      "failure",
      "Release verification failed; inspect workflow logs"
    );
    throw error;
  }
}
try {
  if (!/^[\w.-]+\/[\w.-]+$/.test(repo || ""))
    throw new Error("GITHUB_REPOSITORY is required");
  const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, "utf8"));
  if (process.env.GITHUB_EVENT_NAME === "pull_request_target")
    verify(event.number);
  else if (process.env.GITHUB_EVENT_NAME === "workflow_run") {
    const branch = event.workflow_run.head_branch;
    if (!/^release\/\d+\.\d+\.\d+$/.test(branch))
      throw new Error("Unexpected release run branch");
    const prs = JSON.parse(
      gh(
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
        "number"
      )
    );
    for (const pr of prs) verify(pr.number);
  } else throw new Error("Unexpected verification event");
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
