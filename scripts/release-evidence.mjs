// Read the build record attached to a release and verify its GitHub run.
import { execFileSync } from "node:child_process";
import { releaseTagName } from "./release-tag.mjs";

export function readVerifiedRelease(version, commit, branch, cwd) {
  const run = (command, args) =>
    execFileSync(command, args, { cwd, encoding: "utf8" }).trim();
  const repo = process.env.GITHUB_REPOSITORY;
  if (!/^[\w.-]+\/[\w.-]+$/.test(repo || ""))
    throw new Error("GITHUB_REPOSITORY is required");
  const tag = releaseTagName(version);
  const record = JSON.parse(
    run("gh", [
      "release",
      "download",
      tag,
      "--repo",
      repo,
      "--pattern",
      "release.json",
      "--output",
      "-",
    ])
  );
  const mode = version.includes("-rc.") ? "rc" : "release";
  if (
    record.mode !== mode ||
    record.version !== version ||
    record.tag !== tag ||
    record.commit !== commit ||
    record.branch !== branch ||
    !/^\d+$/.test(record.runId || "") ||
    !/^\d+$/.test(record.runAttempt || "")
  )
    throw new Error(
      "Release build record does not identify this branch and commit"
    );
  const git = (...args) => run("git", args);
  if (
    git("rev-parse", `refs/tags/${tag}^{commit}`) !== commit ||
    git("rev-parse", `refs/tags/${tag}`) !== record.tagObject ||
    git("rev-parse", `${commit}^{tree}`) !== record.tree ||
    git("rev-list", "--parents", "-n", "1", commit) !==
      `${commit} ${record.start}`
  )
    throw new Error("Release build record does not match the tagged source");
  const result = JSON.parse(
    run("gh", ["api", `repos/${repo}/actions/runs/${record.runId}`])
  );
  const workflow =
    mode === "rc"
      ? record.preparation === "start"
        ? "start-release.yml"
        : "prepare-rc.yml"
      : "prepare-release.yml";
  if (
    !Number.isInteger(result.run_attempt) ||
    result.run_attempt < Number(record.runAttempt) ||
    result.conclusion !== "success" ||
    result.status !== "completed" ||
    result.event !== "workflow_dispatch" ||
    result.path !== `.github/workflows/${workflow}` ||
    result.head_sha !== record.start ||
    result.head_branch !== record.sourceRef?.replace(/^refs\/heads\//, "") ||
    result.repository?.full_name !== repo
  )
    throw new Error("The matching release workflow has not succeeded");
  return record;
}

export function validateMainProtection(cwd) {
  const [owner, name] = (process.env.GITHUB_REPOSITORY || "").split("/");
  const query =
    'query($owner:String!,$name:String!){repository(owner:$owner,name:$name){autoMergeAllowed mergeCommitAllowed deleteBranchOnMerge ref(qualifiedName:"refs/heads/main"){branchProtectionRule{requiresStatusChecks requiresStrictStatusChecks requiredStatusCheckContexts}}}}';
  const response = JSON.parse(
    execFileSync(
      "gh",
      [
        "api",
        "graphql",
        "-f",
        `query=${query}`,
        "-f",
        `owner=${owner}`,
        "-f",
        `name=${name}`,
      ],
      { cwd, encoding: "utf8" }
    )
  );
  const repository = response.data?.repository;
  if (
    !repository?.autoMergeAllowed ||
    !repository.mergeCommitAllowed ||
    repository.deleteBranchOnMerge
  )
    throw new Error(
      "Enable auto-merge and merge commits, and disable automatic branch deletion in repository settings; release branches must be retained"
    );
  const rule = repository.ref?.branchProtectionRule;
  if (
    !rule?.requiresStatusChecks ||
    !rule.requiresStrictStatusChecks ||
    !rule.requiredStatusCheckContexts.includes("Release PR verification")
  )
    throw new Error(
      "Configure main branch protection to require Release PR verification and up-to-date branches before enabling release auto-merge"
    );
}
