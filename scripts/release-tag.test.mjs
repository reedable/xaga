import assert from "node:assert/strict";
import test from "node:test";
import {
  releaseTagName,
  validateCleanStatus,
  validateVersionRecords,
} from "./release-tag.mjs";

test("creates final and release-candidate tag names", () => {
  assert.equal(releaseTagName("0.9.0"), "v0.9.0");
  assert.equal(releaseTagName("0.9.0-rc.1"), "v0.9.0-rc.1");
  assert.equal(releaseTagName("0.9.0-rc.12"), "v0.9.0-rc.12");
});

test("rejects development and unsupported release versions", () => {
  assert.throws(
    () => releaseTagName("0.9.0-dev"),
    /Development version 0\.9\.0-dev cannot be tagged.*version:rc.*version:release/
  );
  assert.throws(() => releaseTagName("0.9.0-rc.0"), /final or a numbered/);
  assert.throws(() => releaseTagName("0.9.0+alpha.abc"), /final or a numbered/);
});

test("requires a clean working tree", () => {
  assert.doesNotThrow(() => validateCleanStatus(""));
  assert.throws(
    () => validateCleanStatus(" M package.json\n"),
    /must be clean/
  );
  assert.throws(() => validateCleanStatus("?? local.txt\n"), /must be clean/);
});

test("requires every recorded application version to match", () => {
  const synchronized = [
    { label: "tauri", version: "0.9.0-rc.1" },
    { label: "npm", version: "0.9.0-rc.1" },
    { label: "cargo", version: "0.9.0-rc.1" },
  ];
  assert.equal(validateVersionRecords(synchronized), "0.9.0-rc.1");
  assert.throws(
    () =>
      validateVersionRecords([
        ...synchronized,
        { label: "lock", version: "0.8.5-dev" },
      ]),
    /Application version is 0\.9\.0-rc\.1.*lock=0\.8\.5-dev/
  );
});
