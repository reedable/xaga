import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const source = dirname(fileURLToPath(import.meta.url));
function fixture(t, version = "0.17.6") {
  const directory = mkdtempSync(join(tmpdir(), "xaga-version-bump-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const root = join(directory, "checkout");
  const remote = join(directory, "origin.git");
  const bin = join(directory, "bin");
  mkdirSync(root);
  mkdirSync(bin);
  const write = (path, value) => {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), value);
  };
  write(
    "package.json",
    JSON.stringify({
      version,
      scripts: Object.fromEntries(
        ["major", "minor", "patch"].map((bump) => [
          `version:${bump}`,
          `node scripts/bump-version.mjs ${bump}`,
        ])
      ),
    })
  );
  write("src-tauri/tauri.conf.json", JSON.stringify({ version }));
  const crates = [
    ["src-tauri", "xaga"],
    ["crates/xaga-core", "xaga-core"],
    ["crates/xaga-cli", "xaga-cli"],
  ];
  for (const [path, name] of crates)
    write(
      `${path}/Cargo.toml`,
      `[package]\nname = "${name}"\nversion = "${version}"\n`
    );
  write(
    "Cargo.lock",
    crates
      .map(
        ([, name]) => `[[package]]\nname = "${name}"\nversion = "${version}"\n`
      )
      .join("\n")
  );
  mkdirSync(join(root, "scripts"));
  for (const name of [
    "prepare-version-bump.mjs",
    "bump-version.mjs",
    "release-tag.mjs",
  ])
    copyFileSync(join(source, name), join(root, "scripts", name));
  const git = (...args) =>
    execFileSync("git", args, {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  git("init", "--initial-branch=main");
  git("config", "user.name", "Fixture");
  git("config", "user.email", "fixture@example.test");
  git("config", "commit.gpgsign", "false");
  git("add", ".");
  git("commit", "-m", "Initial source");
  git("init", "--bare", "--initial-branch=main", remote);
  git("remote", "add", "origin", remote);
  git("push", "origin", "main");
  const start = git("rev-parse", "HEAD");
  const state = join(directory, "pr.json");
  writeFileSync(
    join(bin, "gh"),
    `#!${process.execPath}
const fs = require('node:fs');
const args = process.argv.slice(2);
const state = process.env.PR_STATE;
if (args[1] === 'list') console.log(fs.existsSync(state) ? fs.readFileSync(state,'utf8') : '[]');
else if (args[1] === 'create') {
 if (process.env.FAIL_PR) process.exit(17);
 fs.writeFileSync(state, JSON.stringify([{url:'https://example.test/pr/1',args}]));
 console.log('https://example.test/pr/1');
} else process.exit(19);
`,
    { mode: 0o755 }
  );
  const summary = join(directory, "summary.md");
  const run = (bump = "patch", extra = {}) =>
    spawnSync(process.execPath, ["scripts/prepare-version-bump.mjs", bump], {
      cwd: root,
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: `${bin}:${process.env.PATH}`,
        GITHUB_REF: "refs/heads/main",
        GITHUB_STEP_SUMMARY: summary,
        PR_STATE: state,
        ...extra,
      },
    });
  return { root, git, write, start, state, summary, run };
}
function succeeds(result) {
  assert.equal(result.status, 0, result.stdout + result.stderr);
}
function fails(result, pattern) {
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, pattern);
}
for (const [bump, next] of [
  ["patch", "0.17.7-dev"],
  ["minor", "0.18.0-dev"],
  ["major", "1.0.0-dev"],
]) {
  test(`${bump} opens a synchronized development PR without advancing main or creating tags`, (t) => {
    const f = fixture(t);
    succeeds(f.run(bump));
    assert.equal(
      f.git("ls-remote", "origin", "refs/heads/main").split(/\s+/)[0],
      f.start
    );
    assert.equal(f.git("ls-remote", "origin", "refs/tags/*"), "");
    const ref = `refs/heads/version/${next}`;
    assert.ok(f.git("ls-remote", "origin", ref));
    assert.equal(f.git("rev-parse", "HEAD^"), f.start);
    const records = ["package.json", "src-tauri/tauri.conf.json"];
    for (const path of records)
      assert.equal(JSON.parse(readFileSync(join(f.root, path))).version, next);
    for (const path of [
      "src-tauri/Cargo.toml",
      "crates/xaga-core/Cargo.toml",
      "crates/xaga-cli/Cargo.toml",
      "Cargo.lock",
    ])
      assert.ok(
        [
          ...readFileSync(join(f.root, path), "utf8").matchAll(
            /version = "([^"]+)"/g
          ),
        ].every((match) => match[1] === next)
      );
    const [pr] = JSON.parse(readFileSync(f.state));
    assert.equal(pr.args[pr.args.indexOf("--base") + 1], "main");
    assert.equal(pr.args[pr.args.indexOf("--head") + 1], `version/${next}`);
    if (bump !== "patch")
      assert.match(readFileSync(f.summary, "utf8"), /maintenance/);
  });
}
for (const [reason, setup, bump, extra, error] of [
  ["invalid bump", () => {}, "rc", {}, /Expected major/],
  [
    "non-main ref",
    () => {},
    "patch",
    { GITHUB_REF: "refs/heads/release/0.17.6" },
    /on main/,
  ],
  [
    "dirty source",
    (f) => f.write("extra.txt", "work"),
    "patch",
    {},
    /clean checkout/,
  ],
  [
    "stale main",
    (f) => {
      f.git("commit", "--allow-empty", "-m", "Later");
      f.git("push", "origin", "main");
      f.git("checkout", f.start);
    },
    "patch",
    {},
    /main moved/,
  ],
  [
    "mismatched versions",
    (f) => {
      f.write("package.json", '{"version":"0.99.0"}');
      f.git("add", ".");
      f.git("commit", "-m", "Mismatch");
      f.git("push", "origin", "main");
    },
    "patch",
    {},
    /records differ/,
  ],
]) {
  test(`rejects ${reason} without pushing a version branch`, (t) => {
    const f = fixture(t);
    setup(f);
    fails(f.run(bump, extra), error);
    assert.equal(f.git("ls-remote", "origin", "refs/heads/version/*"), "");
  });
}
test("rejects RC versions", (t) => {
  const f = fixture(t, "0.17.6-rc.1");
  fails(f.run(), /release candidate/);
  assert.equal(f.git("ls-remote", "origin", "refs/heads/version/*"), "");
});
test("retargets an existing development version through the CLI rules", (t) => {
  const f = fixture(t, "0.17.7-dev");
  succeeds(f.run("minor"));
  assert.equal(
    JSON.parse(readFileSync(join(f.root, "package.json"))).version,
    "0.18.0-dev"
  );
});
test("recovers failed PR creation and reuses the PR without overwriting the branch", (t) => {
  const f = fixture(t);
  fails(f.run("patch", { FAIL_PR: "1" }), /Command failed/);
  const ref = "refs/heads/version/0.17.7-dev";
  const before = f.git("ls-remote", "origin", ref);
  f.git("checkout", "main");
  succeeds(f.run());
  const state = readFileSync(f.state, "utf8");
  f.git("checkout", "main");
  succeeds(f.run());
  assert.equal(f.git("ls-remote", "origin", ref), before);
  assert.equal(readFileSync(f.state, "utf8"), state);
});
test("refuses to replace existing branch work", (t) => {
  const f = fixture(t);
  succeeds(f.run());
  f.write("release-fix.txt", "preserve");
  f.git("add", ".");
  f.git("commit", "-m", "Additional work");
  f.git("push", "origin", "HEAD:refs/heads/version/0.17.7-dev");
  const before = f.git("ls-remote", "origin", "refs/heads/version/*");
  f.git("checkout", "main");
  fails(f.run(), /already exists/);
  assert.equal(f.git("ls-remote", "origin", "refs/heads/version/*"), before);
});
