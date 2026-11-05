// Builds an ordinary local artifact without changing durable version files.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { alphaIcons, generateAlphaIcons, ROOT } from "./alpha-icons.mjs";

export function alphaVersion(version) {
  if (typeof version !== "string" || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(`Alpha builds require an application version without build metadata; got ${JSON.stringify(version)}.`);
  }
  return `${version}+alpha`;
}

export function tauriBuildArgs(version, extraArgs = [], icons = alphaIcons) {
  return [
    "run", "tauri", "--", "build", "--config",
    JSON.stringify({ version, bundle: { icon: icons } }),
    ...extraArgs,
  ];
}

function main() {
  const { version } = JSON.parse(readFileSync(resolve(ROOT, "src-tauri/tauri.conf.json"), "utf8"));
  const buildVersion = alphaVersion(version);
  console.log(`Building Xaga ${buildVersion}`);
  generateAlphaIcons();
  const result = spawnSync("npm", tauriBuildArgs(buildVersion, process.argv.slice(2)), {
    cwd: ROOT,
    stdio: "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exitCode = result.status ?? 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
