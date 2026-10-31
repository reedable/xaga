// Native macOS probe; all network destinations are a controlled local server
// or .invalid hosts rejected by the deny-all proxy. Requires a GUI session.
import { build } from "esbuild";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const directory = await mkdtemp(path.join(tmpdir(), "xaga-svg-probe-"));
try {
  const output = path.join(directory, "probe.js");
  await build({
    entryPoints: ["src-tauri/tests/fixtures/svg_probe.ts"],
    bundle: true,
    format: "iife",
    outfile: output,
    plugins: [
      {
        name: "inline-reader-css",
        setup(builder) {
          builder.onResolve({ filter: /\.css\?inline$/ }, async (args) => {
            const resolved = await builder.resolve(args.path.replace(/\?inline$/, ""), {
              resolveDir: args.resolveDir,
              kind: args.kind,
            });
            return { ...resolved, namespace: "reader-css" };
          });
          builder.onLoad(
            { filter: /.*/, namespace: "reader-css" },
            async (args) => {
              const css = await build({
                entryPoints: [args.path],
                bundle: true,
                write: false,
              });
              return { contents: css.outputFiles[0].text, loader: "text" };
            }
          );
        },
      },
    ],
  });
  // Read now so a failed fixture build cannot silently leave an empty probe.
  await readFile(output);
  const result = spawnSync(
    "cargo",
    ["test", "-p", "xaga", "--test", "svg_webview", "--offline"],
    {
      stdio: "inherit",
      env: { ...process.env, XAGA_SVG_PROBE_SCRIPT: output },
    }
  );
  process.exitCode = result.status ?? 1;
} finally {
  await rm(directory, { recursive: true, force: true });
}
