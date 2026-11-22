import assert from "node:assert/strict";
import { test } from "node:test";
import { checks, verify } from "./verify-completion.mjs";

test("completion gate runs every check and refuses Rust warnings", () => {
  const calls = [];
  assert.equal(
    verify((command, args, options) => {
      calls.push([command, args]);
      assert.match(options.env.RUSTFLAGS, /-D warnings/);
      assert.match(options.env.RUSTDOCFLAGS, /-D warnings/);
      return { status: 0 };
    }),
    0
  );
  assert.deepEqual(calls, checks);
  assert.deepEqual(calls.slice(-4), [
    ["npm", ["run", "test:browser"]],
    ["npm", ["run", "check:rust"]],
    ["npm", ["run", "test:headless"]],
    ["npm", ["run", "test:gui"]],
  ]);
});

test("completion gate stops on a failed check or missing executable", () => {
  for (const failure of [
    { status: 7 },
    { status: null, error: new Error("fixture unavailable") },
  ]) {
    let calls = 0;
    assert.notEqual(
      verify(() => {
        calls++;
        return failure;
      }),
      0
    );
    assert.equal(calls, 1);
  }
});
