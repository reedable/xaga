import assert from "node:assert/strict";
import test from "node:test";
import {
  betaVersion,
  tauriBuildArgs,
  validateCleanStatus,
  validateUnpublishedVersion,
} from "./build-beta.mjs";

test("adds beta metadata containing the commit hash", () => {
  assert.equal(
    betaVersion("0.8.4-rc.2", "f62fb2f"),
    "0.8.4-rc.2+beta.f62fb2f"
  );
});

test("allows final release versions with beta metadata", () => {
  assert.equal(betaVersion("0.14.0", "f62fb2f"), "0.14.0+beta.f62fb2f");
});

test("rejects development and unsupported prerelease versions", () => {
  for (const version of ["0.8.4-dev", "0.8.4-beta.1", "0.8.4-rc", "0.8.4-rc.0", "0.8.4-rc.01", "0.8.4+beta.abc"]) {
    assert.throws(() => betaVersion(version, "f62fb2f"), /release or numbered release-candidate version/);
  }
});

test("accepts untagged RC and final versions", () => {
  for (const version of ["0.14.0-rc.2", "0.14.0"]) {
    assert.doesNotThrow(() => validateUnpublishedVersion(version, false));
  }
});

test("rejects tagged RC and final versions", () => {
  for (const version of ["0.14.0-rc.2", "0.14.0"]) {
    assert.throws(() => validateUnpublishedVersion(version, true), /already has release tag.*untagged version/);
  }
});

test("requires a hexadecimal commit hash", () => {
  assert.throws(
    () => betaVersion("0.8.4-rc.2", "comparison-a"),
    /invalid commit hash/
  );
});

test("passes the beta version to Tauri across npm's argument boundary", () => {
  assert.deepEqual(
    tauriBuildArgs("0.8.5-rc.2+beta.5f97d65", ["/tmp/alpha/icon.icns"]),
    [
      "run",
      "tauri",
      "--",
      "build",
      "--config",
      '{"version":"0.8.5-rc.2+beta.5f97d65","bundle":{"icon":["/tmp/alpha/icon.icns"]}}',
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
