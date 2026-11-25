// Report the release result on the prepared commit, not the dispatch commit.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export function reportReleaseStatus(identity, state) {
  // Report only after the prepared commit reaches the release branch or tag.
  const refs = execFileSync(
    "git",
    [
      "ls-remote",
      "origin",
      ...(identity.branch ? [identity.branch] : []),
      `refs/tags/${identity.tag}`,
      `refs/tags/${identity.tag}^{}`,
    ],
    { encoding: "utf8" }
  );
  if (
    !refs.split("\n").some((line) => line.split(/\s+/)[0] === identity.commit)
  )
    return;
  const description = {
    pending: "Release verification, build, and publication are running",
    success: "Release verification, build, and publication succeeded",
    failure: "Release workflow failed or was cancelled",
  }[state];
  writeStatus(identity.commit, state, description, identity.runId);
}

function writeStatus(commit, state, description, fallbackRunId) {
  const repo = process.env.GITHUB_REPOSITORY;
  const runId = process.env.GITHUB_RUN_ID || fallbackRunId;
  if (
    !/^[\w.-]+\/[\w.-]+$/.test(repo || "") ||
    !/^[0-9a-f]{40}$/.test(commit || "") ||
    !/^\d+$/.test(runId || "") ||
    !["pending", "success", "failure"].includes(state)
  )
    throw new Error("Invalid release status identity or state");
  execFileSync(
    "gh",
    [
      "api",
      "--method",
      "POST",
      `repos/${repo}/statuses/${commit}`,
      "-f",
      "context=Release build",
      "-f",
      `state=${state}`,
      "-f",
      `description=${description}`,
      "-f",
      `target_url=https://github.com/${repo}/actions/runs/${runId}`,
    ],
    { stdio: ["ignore", "pipe", "pipe"] }
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    const [path, state] = process.argv.slice(2);
    // Preparation/download can fail before a commit identity is available.
    if (path && existsSync(path))
      reportReleaseStatus(JSON.parse(readFileSync(path, "utf8")), state);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
