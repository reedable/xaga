import assert from "node:assert/strict";
import test from "node:test";
import { cargoMetadataArgs, refreshCargoLock } from "./refresh-cargo-lock.mjs";

test("refreshes Cargo.lock through metadata without compiling", () => {
  let invocation;
  refreshCargoLock((command, args, options) => {
    invocation = { command, args, options };
    return { status: 0 };
  });

  assert.equal(invocation.command, "cargo");
  assert.deepEqual(invocation.args, [
    "metadata",
    "--format-version",
    "1",
    "--no-deps",
  ]);
  assert.deepEqual(cargoMetadataArgs, invocation.args);
  assert.deepEqual(invocation.options.stdio, ["ignore", "ignore", "inherit"]);
});

test("fails when Cargo cannot refresh the lockfile", () => {
  assert.throws(
    () => refreshCargoLock(() => ({ status: 101 })),
    /Cargo.lock refresh failed with exit code 101/
  );
  assert.throws(
    () => refreshCargoLock(() => ({ error: new Error("cargo unavailable") })),
    /cargo unavailable/
  );
});
