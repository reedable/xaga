import { spawnSync } from "node:child_process";

if (process.platform !== "darwin") {
  console.error("The native WebKit suite requires macOS with a desktop session.");
  process.exit(1);
}

function run(command, args, extraEnv = {}) {
  console.log(`\n> ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, {
    stdio: "inherit",
    env: { ...process.env, ...extraEnv },
  });
  if (result.error) console.error(result.error.message);
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function nativeTest(name, env = {}) {
  run("cargo", ["test", "-p", "xaga", "--test", name, "--locked"], env);
}

for (const name of [
  "webview_policy",
  "ui_network_webview",
  "loader_loopback_webview",
  "feed_navigation_webview",
  "loader_lifecycle_webview",
]) {
  nativeTest(name);
}

run(process.execPath, ["scripts/verify-svg-webview.mjs"]);

for (const scenario of [
  "icon", "image", "image-document", "svg", "svg-document",
  "svg-icon-document", "avif", "avif-document", "avif-icon-document",
  "pdf", "invalid-pdf", "favicon-pdf", "html", "blob-pdf",
]) {
  nativeTest("resource_download_webview", { XAGA_DOWNLOAD_SMOKE_CASE: scenario });
}

console.log("\nThe fingerprint probe opens a local page in Safari.");
nativeTest("fingerprint_webview");
