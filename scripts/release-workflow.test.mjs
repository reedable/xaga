import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const source = dirname(fileURLToPath(import.meta.url));
function fixture(t, version = "0.17.3-dev") {
  const directory = realpathSync(mkdtempSync(join(tmpdir(), "xaga-release-")));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const root = join(directory, "checkout");
  const remote = join(directory, "origin.git");
  const bin = join(directory, "bin");
  mkdirSync(root);
  mkdirSync(bin);
  const write = (path, contents) => {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), contents);
  };
  for (const name of [
    "bump-version.mjs",
    "release-tag.mjs",
    "prepare-release.mjs",
    "publish-release.mjs",
  ]) {
    mkdirSync(join(root, "scripts"), { recursive: true });
    copyFileSync(join(source, name), join(root, "scripts", name));
  }
  write(
    "package.json",
    JSON.stringify(
      {
        version,
        scripts: {
          "check:uncommitted": "git diff --quiet && git diff --cached --quiet",
          "version:rc":
            "npm run check:uncommitted && node scripts/bump-version.mjs rc",
          "version:release":
            "npm run check:uncommitted && node scripts/bump-version.mjs release",
          "release:tag": "node scripts/release-tag.mjs",
        },
      },
      null,
      2
    )
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
  write(".gitignore", "target/\n");
  write("application.txt", "original source\n");
  const git = (...args) =>
    execFileSync("git", args, {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  git("init", "--initial-branch=main");
  git("config", "commit.gpgsign", "false");
  git("config", "tag.gpgsign", "false");
  git("config", "user.name", "Fixture");
  git("config", "user.email", "fixture@example.test");
  git("add", ".");
  git("commit", "-m", "Initial source");
  git("init", "--bare", "--initial-branch=main", remote);
  git("remote", "add", "origin", remote);
  git("push", "origin", "main");
  const start = git("rev-parse", "HEAD");
  const quote = (value) => "'" + value.replaceAll("'", "'\\''") + "'";
  const mockPath = join(directory, "gh.cjs");
  writeFileSync(
    mockPath,
    `
const fs = require('node:fs');
const path = process.env.GH_STATE;
const state = fs.existsSync(path) ? JSON.parse(fs.readFileSync(path)) : { releases: [], uploads: [] };
const args = process.argv.slice(2);
if (process.env.GH_FAIL === args[1]) process.exit(17);
if (args[0] === 'api') console.log(JSON.stringify([state.releases]));
else if (args[1] === 'create') state.releases.push({tag_name: args[2], draft: true, prerelease: args.includes('--prerelease')});
else if (args[1] === 'upload') state.uploads.push(args[2]);
else process.exit(19);
fs.writeFileSync(path, JSON.stringify(state));
`
  );
  writeFileSync(
    join(bin, "gh"),
    `#!/bin/sh\nexec ${quote(process.execPath)} ${quote(mockPath)} "$@"\n`,
    { mode: 0o755 }
  );
  writeFileSync(
    join(bin, "codesign"),
    '#!/bin/sh\nexit "${SIGNATURE_STATUS:-0}"\n',
    { mode: 0o755 }
  );
  const env = {
    ...process.env,
    PATH: `${bin}:${process.env.PATH}`,
    GITHUB_REF: "refs/heads/main",
    GITHUB_REPOSITORY: "fixture/xaga",
    RELEASE_STATE: join(directory, "state.json"),
    RELEASE_PACKAGE: join(directory, "package"),
    GH_STATE: join(directory, "gh.json"),
  };
  delete env.GITHUB_STEP_SUMMARY;
  const command = (script, args = [], extra = {}) =>
    spawnSync(process.execPath, [join(root, "scripts", script), ...args], {
      cwd: root,
      env: { ...env, ...extra },
      encoding: "utf8",
    });
  const prepare = (mode = "rc", extra) =>
    command("prepare-release.mjs", ["prepare", mode], extra);
  const build = () => {
    write(
      "target/aarch64-apple-darwin/release/bundle/macos/Xaga.app/Contents/MacOS/Xaga",
      "fixture executable"
    );
    write(
      "target/aarch64-apple-darwin/release/bundle/dmg/Xaga.dmg",
      "fixture disk image"
    );
  };
  const seal = (extra) => command("prepare-release.mjs", ["seal"], extra);
  const publish = (extra) => command("publish-release.mjs", [], extra);
  const refs = () => git("ls-remote", "origin");
  const manifest = () =>
    JSON.parse(readFileSync(join(env.RELEASE_PACKAGE, "release.json"), "utf8"));
  return {
    root,
    remote,
    directory,
    git,
    write,
    start,
    env,
    prepare,
    build,
    seal,
    publish,
    refs,
    manifest,
  };
}
function succeeds(result) {
  assert.equal(result.status, 0, result.stderr + result.stdout);
}
function fails(result) {
  assert.notEqual(result.status, 0, result.stderr + result.stdout);
}

for (const [before, mode, after] of [
  ["0.17.3-dev", "rc", "0.17.3-rc.1"],
  ["0.17.3-rc.2", "rc", "0.17.3-rc.3"],
  ["0.17.3-dev", "release", "0.17.3"],
  ["0.17.3-rc.2", "release", "0.17.3"],
]) {
  test(`${mode} prepares ${before} as ${after} without committing or pushing`, (t) => {
    const f = fixture(t, before);
    const refs = f.refs();
    succeeds(f.prepare(mode));
    assert.equal(JSON.parse(readFileSync(f.env.RELEASE_STATE)).version, after);
    assert.equal(f.git("rev-parse", "HEAD"), f.start);
    assert.equal(f.git("tag", "--list"), "");
    assert.equal(f.refs(), refs);
    fails(f.prepare(mode)); // A retry needs a fresh checkout.
  });
}

test("rejects final versions, tag dispatch, and dirty checkouts before preparation", (t) => {
  const f = fixture(t, "0.17.3");
  fails(f.prepare("rc"));
  fails(f.prepare("release"));
  fails(f.prepare("rc", { GITHUB_REF: "refs/tags/v0.17.3" }));
  f.write("untracked", "work");
  fails(f.prepare());
  assert.equal(f.git("rev-parse", "HEAD"), f.start);
});

test("build and signature failures leave local commits and remote refs unchanged", (t) => {
  const f = fixture(t);
  const refs = f.refs();
  succeeds(f.prepare());
  fails(f.seal()); // No successful build output.
  f.build();
  fails(f.seal({ SIGNATURE_STATUS: "1" }));
  assert.equal(f.git("rev-parse", "HEAD"), f.start);
  assert.equal(f.refs(), refs);
  assert.equal(f.git("tag", "--list"), "");
});

test("refuses source changes made during the build", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  f.write("application.txt", "different source");
  fails(f.seal());
  f.git("add", "application.txt");
  fails(f.seal());
  assert.equal(f.git("rev-parse", "HEAD"), f.start);
});

test("publishes the built tree and retries failed uploads without another commit or tag", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  const state = f.manifest();
  assert.equal(f.git("rev-parse", "HEAD^{tree}"), state.tree);
  assert.equal(f.git("rev-parse", "HEAD^"), f.start);
  fails(f.publish({ GH_FAIL: "upload" }));
  const refs = f.refs();
  assert.ok(refs.includes(state.commit));
  assert.ok(refs.includes(state.tagObject));
  succeeds(f.publish());
  assert.equal(f.refs(), refs);
  const gh = JSON.parse(readFileSync(f.env.GH_STATE));
  assert.equal(gh.releases.length, 1);
  assert.equal(gh.releases[0].prerelease, true);
  assert.deepEqual(gh.uploads, [state.tag]);
});

test("final releases produce drafts without prerelease status", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  succeeds(f.prepare("release"));
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  const gh = JSON.parse(readFileSync(f.env.GH_STATE));
  assert.equal(gh.releases[0].draft, true);
  assert.equal(gh.releases[0].prerelease, false);
});

test("branch movement rejects both updates", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  f.git("checkout", "--detach", f.start);
  f.write("application.txt", "intervening change");
  f.git("add", "application.txt");
  f.git("commit", "-m", "Intervening change");
  f.git("push", "origin", "HEAD:main");
  const refs = f.refs();
  fails(f.publish());
  assert.equal(f.refs(), refs);
  assert.equal(existsSync(f.env.GH_STATE), false);
});

test("an existing tag prevents updating the branch", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  const state = f.manifest();
  f.git("push", "origin", `${f.start}:refs/tags/${state.tag}`);
  const refs = f.refs();
  fails(f.publish());
  assert.equal(f.refs(), refs);
});

test("server rejection of a tag leaves the branch unchanged atomically", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  writeFileSync(
    join(f.remote, "hooks/update"),
    '#!/bin/sh\ncase "$1" in refs/tags/*) exit 1;; esac\n',
    { mode: 0o755 }
  );
  const refs = f.refs();
  fails(f.publish());
  assert.equal(f.refs(), refs);
});

test("artifact corruption is rejected before any remote update", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  writeFileSync(
    join(f.env.RELEASE_PACKAGE, f.manifest().assets[0].name),
    "corrupted"
  );
  const refs = f.refs();
  fails(f.publish());
  assert.equal(f.refs(), refs);
});

test("manual tagged builds preserve refs and use the same draft publication", (t) => {
  const f = fixture(t, "0.17.3");
  f.git("tag", "-a", "v0.17.3", "-m", "Release");
  f.git("push", "origin", "v0.17.3");
  const refs = f.refs();
  succeeds(f.prepare("tag", { GITHUB_REF: "refs/tags/v0.17.3" }));
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  assert.equal(f.refs(), refs);
  assert.equal(f.git("rev-parse", "HEAD"), f.start);
});

test("a branch race during push is rejected by the server", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  const other = f.git(
    "commit-tree",
    f.git("rev-parse", `${f.start}^{tree}`),
    "-p",
    f.start,
    "-m",
    "Racing change"
  );
  f.git("--git-dir", f.remote, "fetch", f.root, other);
  const quote = (value) => "'" + value.replaceAll("'", "'\\''") + "'";
  writeFileSync(
    join(f.root, ".git/hooks/pre-push"),
    `#!/bin/sh\ngit --git-dir=${quote(f.remote)} update-ref refs/heads/main ${other} ${f.start}\n`,
    { mode: 0o755 }
  );
  fails(f.publish());
  assert.equal(
    f.git("ls-remote", "origin", "refs/heads/main"),
    `${other}\trefs/heads/main`
  );
  assert.equal(f.git("ls-remote", "origin", "refs/tags/*"), "");
});

test("competing RC and final preparation cannot both update the same starting branch", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  const rc = f.manifest();
  f.git("reset", "--hard", f.start);
  succeeds(f.prepare("release"));
  const finalPackage = join(f.directory, "final-package");
  succeeds(f.seal({ RELEASE_PACKAGE: finalPackage }));
  succeeds(f.publish());
  const refs = f.refs();
  fails(f.publish({ RELEASE_PACKAGE: finalPackage }));
  assert.equal(f.refs(), refs);
  assert.ok(refs.includes(rc.tagObject));
});

test("published releases are never overwritten during recovery", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  const gh = JSON.parse(readFileSync(f.env.GH_STATE));
  gh.releases[0].draft = false;
  writeFileSync(f.env.GH_STATE, JSON.stringify(gh));
  fails(f.publish());
  assert.deepEqual(JSON.parse(readFileSync(f.env.GH_STATE)), gh);
});

test("publication recovers in a fresh checkout using only the retained package", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  const checkout = join(f.directory, "publisher");
  f.git("clone", f.remote, checkout);
  const result = spawnSync(
    process.execPath,
    [join(checkout, "scripts/publish-release.mjs")],
    { cwd: checkout, env: f.env, encoding: "utf8" }
  );
  succeeds(result);
  assert.ok(f.refs().includes(f.manifest().commit));
});

test("selected emergency branch is updated while the default branch is untouched", (t) => {
  const f = fixture(t);
  f.git("switch", "-c", "emergency/patch");
  f.git("push", "origin", "emergency/patch");
  succeeds(f.prepare("rc", { GITHUB_REF: "refs/heads/emergency/patch" }));
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  assert.equal(
    f.git("ls-remote", "origin", "refs/heads/main"),
    `${f.start}\trefs/heads/main`
  );
  assert.equal(
    f.git("ls-remote", "origin", "refs/heads/emergency/patch"),
    `${f.manifest().commit}\trefs/heads/emergency/patch`
  );
});

test("draft creation failure can be retried after the refs have been pushed", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  fails(f.publish({ GH_FAIL: "create" }));
  const refs = f.refs();
  succeeds(f.publish());
  assert.equal(f.refs(), refs);
  assert.equal(JSON.parse(readFileSync(f.env.GH_STATE)).releases.length, 1);
});

test("an identical tag without the branch update is not successful push recovery", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  const state = f.manifest();
  f.git("push", "origin", `${state.tagObject}:refs/tags/${state.tag}`);
  const refs = f.refs();
  fails(f.publish());
  assert.equal(f.refs(), refs);
  assert.equal(existsSync(f.env.GH_STATE), false);
});
