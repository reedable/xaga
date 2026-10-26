import assert from "node:assert/strict";
import test from "node:test";
import { tauriDevArgs } from "./dev.mjs";

test("passes the Alpha icon to Tauri dev across npm's argument boundary", () => {
  assert.deepEqual(tauriDevArgs(["/tmp/alpha/icon.icns"]), [
    "run",
    "tauri",
    "--",
    "dev",
    "--config",
    '{"bundle":{"icon":["/tmp/alpha/icon.icns"]}}',
  ]);
});
