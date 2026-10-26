import assert from "node:assert/strict";
import test from "node:test";
import { nextVersion, replaceCargoLockVersions } from "./bump-version.mjs";

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

test("updates exactly the Xaga workspace package versions in Cargo.lock", () => {
  const lock = `[[package]]
name = "dependency"
version = "0.9.0"

[[package]]
name = "xaga"
version = "0.8.5-rc.1"

[[package]]
name = "xaga-cli"
version = "0.8.5-rc.1"

[[package]]
name = "xaga-core"
version = "0.8.5-rc.1"
`;
  const updated = replaceCargoLockVersions(lock, "0.8.5-rc.2");
  assert.match(updated, /name = "dependency"\nversion = "0\.9\.0"/);
  assert.equal(updated.match(/version = "0\.8\.5-rc\.2"/g)?.length, 3);
});

test("rejects an incomplete Cargo.lock workspace package set", () => {
  assert.throws(
    () =>
      replaceCargoLockVersions(
        '[[package]]\nname = "xaga"\nversion = "0.8.5-rc.1"\n',
        "0.8.5-rc.2"
      ),
    /Expected 3.*updated 1/
  );
});
