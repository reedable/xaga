import assert from "node:assert/strict";
import test from "node:test";
import { nextVersion } from "./bump-version.mjs";

test("version bumps start a development version", () => {
  assert.equal(nextVersion("0.8.4", "patch"), "0.8.5-dev");
  assert.equal(nextVersion("0.8.4", "minor"), "0.9.0-dev");
  assert.equal(nextVersion("0.8.4", "major"), "1.0.0-dev");
});

test("a development target can be retargeted", () => {
  assert.equal(nextVersion("0.8.5-dev", "patch"), "0.8.6-dev");
  assert.equal(nextVersion("0.8.5-dev", "minor"), "0.9.0-dev");
  assert.equal(nextVersion("0.8.5-dev", "major"), "1.0.0-dev");
});

test("release candidates start at one and increment", () => {
  assert.equal(nextVersion("0.8.5-dev", "rc"), "0.8.5-rc.1");
  assert.equal(nextVersion("0.8.5-rc.1", "rc"), "0.8.5-rc.2");
  assert.equal(nextVersion("0.8.5-rc.9", "rc"), "0.8.5-rc.10");
});

test("release removes either development suffix", () => {
  assert.equal(nextVersion("0.8.5-dev", "release"), "0.8.5");
  assert.equal(nextVersion("0.8.5-rc.2", "release"), "0.8.5");
});

test("rejects invalid lifecycle transitions", () => {
  assert.throws(() => nextVersion("0.8.4", "rc"), /final version/);
  assert.throws(() => nextVersion("0.8.4", "release"), /already a final/);
  assert.throws(() => nextVersion("0.8.5-rc.1", "patch"), /release candidate/);
});

test("rejects unsupported versions and commands", () => {
  assert.throws(
    () => nextVersion("0.8.5+alpha.abc", "patch"),
    /Expected version/
  );
  assert.throws(() => nextVersion("0.8.5-beta.1", "rc"), /Expected version/);
  assert.throws(() => nextVersion("0.8.5", "beta"), /Expected one of/);
});
