import assert from "node:assert/strict";
import test from "node:test";
import { alphaVersion, tauriBuildArgs } from "./build-alpha.mjs";
import { alphaIcons } from "./alpha-icons.mjs";

test("adds exactly ID-free alpha metadata to development, RC, and final versions", () => {
  for (const version of ["0.14.3-dev", "0.14.3-rc.1", "0.14.3"]) {
    assert.equal(alphaVersion(version), `${version}+alpha`);
  }
});

test("rejects existing metadata rather than producing a second suffix", () => {
  for (const version of ["0.14.3+alpha", "0.14.3+beta.abc", undefined]) {
    assert.throws(() => alphaVersion(version), /without build metadata/);
  }
});

test("passes invocation-only version and shared Alpha icons with additional Tauri arguments", () => {
  assert.deepEqual(tauriBuildArgs("0.14.3-dev+alpha", ["--no-bundle"]), [
    "run",
    "tauri",
    "--",
    "build",
    "--config",
    JSON.stringify({
      version: "0.14.3-dev+alpha",
      bundle: { icon: alphaIcons },
    }),
    "--no-bundle",
  ]);
});

test("alpha invocation allows local changes, preserves version files, and propagates build failure", async (t) => {
  const {
    mkdtempSync,
    realpathSync,
    mkdirSync,
    copyFileSync,
    writeFileSync,
    readFileSync,
    rmSync,
  } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");
  const { spawnSync } = await import("node:child_process");
  const root = realpathSync(mkdtempSync(join(tmpdir(), "xaga-alpha-")));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const dir of [
    "scripts",
    "src-tauri",
    "assets",
    "target/alpha-icons",
    "bin",
  ]) {
    mkdirSync(join(root, dir), { recursive: true });
  }
  for (const name of ["build-alpha.mjs", "alpha-icons.mjs"]) {
    copyFileSync(new URL(name, import.meta.url), join(root, "scripts", name));
  }
  const sourceFiles = {
    "src-tauri/tauri.conf.json": '{"version":"0.14.3-dev"}\n',
    "package.json": '{"version":"0.14.3-dev"}\n',
    "Cargo.lock": "local lockfile contents\n",
    "src-tauri/Cargo.toml": '[package]\nversion = "0.14.3-dev"\n',
  };
  for (const [name, contents] of Object.entries(sourceFiles))
    writeFileSync(join(root, name), contents);
  writeFileSync(join(root, "assets/Xaga-iOS-Alpha-1024x1024@1x.png"), "source");
  writeFileSync(join(root, "target/alpha-icons/icon.icns"), "cached icon");
  writeFileSync(join(root, "uncommitted.txt"), "work in progress");
  writeFileSync(join(root, "bin/git"), "#!/bin/sh\nexit 99\n", { mode: 0o755 });
  writeFileSync(
    join(root, "bin/mock.cjs"),
    "require('node:fs').writeFileSync(process.env.CAPTURE, JSON.stringify(process.argv.slice(2))); process.exit(Number(process.env.BUILD_STATUS));\n"
  );
  const quote = (value) => "'" + value.replaceAll("'", "'\\''") + "'";
  writeFileSync(
    join(root, "bin/npm"),
    `#!/bin/sh\nexec ${quote(process.execPath)} ${quote(join(root, "bin/mock.cjs"))} "$@"\n`,
    { mode: 0o755 }
  );
  const capture = join(root, "arguments.json");
  for (const status of [0, 7]) {
    const result = spawnSync(
      process.execPath,
      [join(root, "scripts/build-alpha.mjs"), "--no-bundle"],
      {
        env: {
          ...process.env,
          PATH: `${join(root, "bin")}:${process.env.PATH}`,
          CAPTURE: capture,
          BUILD_STATUS: String(status),
        },
        encoding: "utf8",
      }
    );
    assert.equal(result.status, status, result.stderr);
    const args = JSON.parse(readFileSync(capture, "utf8"));
    assert.deepEqual(args.slice(0, 5), [
      "run",
      "tauri",
      "--",
      "build",
      "--config",
    ]);
    const config = JSON.parse(args[5]);
    assert.equal(config.version, "0.14.3-dev+alpha");
    assert.ok(
      config.bundle.icon.every((icon) =>
        icon.startsWith(join(root, "target/alpha-icons"))
      )
    );
    assert.equal(args[6], "--no-bundle");
    for (const [name, contents] of Object.entries(sourceFiles))
      assert.equal(readFileSync(join(root, name), "utf8"), contents);
  }
});
