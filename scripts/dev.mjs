// Runs the development app with the Alpha icon without changing durable Tauri
// configuration or production icons.

import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { alphaIcons, generateAlphaIcons, ROOT } from "./alpha-icons.mjs";

export function tauriDevArgs(icons = alphaIcons) {
  return [
    "run",
    "tauri",
    "--",
    "dev",
    "--config",
    JSON.stringify({ bundle: { icon: icons } }),
  ];
}

function main() {
  generateAlphaIcons();
  const result = spawnSync("npm", tauriDevArgs(), {
    cwd: ROOT,
    stdio: "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exitCode = result.status ?? 1;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
