// Refreshes Cargo.lock after changing workspace package versions without
// compiling the Rust workspace.

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const cargoMetadataArgs = ["metadata", "--format-version", "1", "--no-deps"];

export function refreshCargoLock(spawn = spawnSync) {
  const result = spawn("cargo", cargoMetadataArgs, {
    cwd: ROOT,
    stdio: ["ignore", "ignore", "inherit"],
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `Cargo.lock refresh failed with exit code ${result.status ?? "unknown"}`
    );
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    refreshCargoLock();
    console.log("Refreshed Cargo.lock workspace versions.");
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
