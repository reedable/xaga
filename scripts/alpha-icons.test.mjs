import assert from "node:assert/strict";
import test from "node:test";
import { isAlphaIconSetCurrent } from "./alpha-icons.mjs";

test("requires Alpha icon generation when the marker is missing", () => {
  assert.equal(isAlphaIconSetCurrent(100, undefined), false);
});

test("requires Alpha icon generation when the source is newer", () => {
  assert.equal(isAlphaIconSetCurrent(101, 100), false);
});

test("skips Alpha icon generation when the marker is current", () => {
  assert.equal(isAlphaIconSetCurrent(100, 100), true);
  assert.equal(isAlphaIconSetCurrent(100, 101), true);
});
