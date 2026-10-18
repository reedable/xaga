import assert from "node:assert/strict";
import test from "node:test";
import { configuredCsp, scriptSrcContainsHash } from "./csp-hash.mjs";

const hash = "'sha256-current='";

test("accepts an exact hash in script-src", () => {
  assert.equal(
    scriptSrcContainsHash(
      `default-src 'self'; script-src 'self' ${hash}`,
      hash
    ),
    true
  );
});

test("rejects a mismatched or unrelated hash", () => {
  assert.equal(
    scriptSrcContainsHash(
      `default-src ${hash}; script-src 'self' 'sha256-stale='`,
      hash
    ),
    false
  );
});

test("rejects a missing script-src hash", () => {
  assert.equal(scriptSrcContainsHash("default-src 'self'", hash), false);
});

test("rejects a missing CSP configuration", () => {
  assert.throws(() => configuredCsp({}), /app\.security\.csp/);
});
