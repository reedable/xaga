import assert from "node:assert/strict";
import test from "node:test";
import {
  alphaVersion,
  tauriBuildArgs,
  validateCleanStatus,
} from "./build-alpha.mjs";

test("adds alpha metadata containing the commit hash", () => {
  assert.equal(
    alphaVersion("0.8.4-rc.2", "f62fb2f"),
    "0.8.4-rc.2+alpha.f62fb2f"
  );
});

test("requires a numbered release-candidate version", () => {
  assert.throws(
    () => alphaVersion("0.8.4", "f62fb2f"),
    /release-candidate version/
  );
  assert.throws(
    () => alphaVersion("0.8.4-dev", "f62fb2f"),
    /release-candidate version/
  );
});

test("requires a hexadecimal commit hash", () => {
  assert.throws(
    () => alphaVersion("0.8.4-rc.2", "comparison-a"),
    /invalid commit hash/
  );
});

test("passes the alpha version to Tauri across npm's argument boundary", () => {
  assert.deepEqual(
    tauriBuildArgs("0.8.5-rc.2+alpha.5f97d65", ["/tmp/alpha/icon.icns"]),
    [
      "run",
      "tauri",
      "--",
      "build",
      "--config",
      '{"version":"0.8.5-rc.2+alpha.5f97d65","bundle":{"icon":["/tmp/alpha/icon.icns"]}}',
    ]
  );
});

test("accepts a clean git status", () => {
  assert.doesNotThrow(() => validateCleanStatus(""));
});

test("rejects tracked, staged, or untracked changes", () => {
  assert.throws(
    () => validateCleanStatus(" M package.json\n"),
    /uncommitted changes/
  );
  assert.throws(
    () => validateCleanStatus("M  package.json\n"),
    /uncommitted changes/
  );
  assert.throws(
    () => validateCleanStatus("?? local.txt\n"),
    /uncommitted changes/
  );
});
