import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import {
  copyFileSync,
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
    "release-status.mjs",
    "prepare-release.mjs",
    "prepare-version-bump.mjs",
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
          "version:patch":
            "npm run check:uncommitted && node scripts/bump-version.mjs patch",
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
  git("commit", "--allow-empty", "-m", "Base");
  git("add", ".");
  git("commit", "-m", "Initial source");
  git("init", "--bare", "--initial-branch=main", remote);
  git("remote", "add", "origin", remote);
  git("push", "origin", "main");
  const start = git("rev-parse", "HEAD");
  const branch = /-rc\.[0-9]+$/.test(version)
    ? `release/${version.replace(/-rc\.[0-9]+$/, "")}`
    : "main";
  if (branch !== "main") {
    git("switch", "-c", branch);
    git("push", "origin", branch);
  }
  const quote = (value) => "'" + value.replaceAll("'", "'\\''") + "'";
  const mockPath = join(directory, "gh.cjs");
  writeFileSync(
    mockPath,
    `
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { execFileSync } = require('node:child_process');
const git = (...args) => execFileSync('git', args, {encoding:'utf8'}).trim();
const path = process.env.GH_STATE;
const state = fs.existsSync(path) ? JSON.parse(fs.readFileSync(path)) : { releases: [], uploads: [], prs: [], merges: [] };
const args = process.argv.slice(2);
if (process.env.GH_FAIL === args[1] || process.env.GH_FAIL === args.slice(0,2).join('-')) process.exit(17);
if (args[0] === 'api' && args[1] === 'graphql') {
 assert.equal(process.env.GH_TOKEN, 'fixture-app-token');
 const { variables } = JSON.parse(fs.readFileSync(0, 'utf8'));
 const { branch, expectedHeadOid, message } = variables.input;
 const ref = "refs/heads/" + branch.branchName;
 const oid = git('commit-tree', git('write-tree'), '-p', expectedHeadOid, '-m', message.headline);
 git('push', '--force-with-lease=' + ref + ':' + expectedHeadOid,
   'origin', oid + ':' + ref);
} else if (args[0] === 'api' && args[3] === 'repos/fixture/xaga/git/refs') {
 assert.equal(process.env.GH_TOKEN, 'fixture-app-token');
 const ref = args[5].slice(4);
 git('push', '--force-with-lease=' + ref + ':', 'origin', args[7].slice(4) + ':' + ref);
} else if (args[0] === 'api' && args.some(arg => arg.includes('/statuses/'))) {
 assert.equal(process.env.GH_TOKEN, 'fixture-workflow-token');
 state.statuses ||= []; state.statuses.push({sha:args.find(arg=>arg.includes('/statuses/')).split('/').at(-1),state:args.find(arg=>arg.startsWith('state=')).slice(6),context:args.find(arg=>arg.startsWith('context=')).slice(8)});
}
else if (args[0] === 'api' && args.includes('--paginate') && args.includes('--slurp')) console.log(JSON.stringify([state.releases]));
else if (args[0] === 'pr' && args[1] === 'list') {
 const head = args[args.indexOf('--head')+1];
 console.log(JSON.stringify(state.prs.filter(pr => pr.head === head)));
}
else if (args[0] === 'pr' && args[1] === 'create') {
 const head = args[args.indexOf('--head')+1];
 assert.equal(process.env.GH_TOKEN, head.startsWith('version/') ? 'fixture-app-token' : 'fixture-workflow-token');
 if (process.env.GH_FAIL_PR_HEAD === head) process.exit(17);
 const number = state.prs.length + 1;
 const url = 'https://example.test/pr/' + number;
 state.prs.push({number, url, base:args[args.indexOf('--base')+1], head});
 console.log(url);
}
else if (args[0] === 'release' && args[1] === 'create') state.releases.push({tag_name: args[2], draft: true, prerelease: args.includes('--prerelease')});
else if (args[0] === 'release' && args[1] === 'upload') state.uploads.push(args[2]);
else { console.error('Unexpected GitHub operation: ' + args.join(' ')); process.exit(19); }
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
    GITHUB_REF: `refs/heads/${branch}`,
    GITHUB_RUN_ID: "100",
    GITHUB_RUN_ATTEMPT: "1",
    GITHUB_REPOSITORY: "fixture/xaga",
    RELEASE_STATE: join(directory, "state.json"),
    RELEASE_PACKAGE: join(directory, "package"),
    GH_STATE: join(directory, "gh.json"),
    GH_TOKEN: "fixture-workflow-token",
    VERSION_BUMP_APP_TOKEN: "fixture-app-token",
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
    branch,
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
    status: (state, path = env.RELEASE_STATE) =>
      command("release-status.mjs", [path, state]),
  };
}
function succeeds(result) {
  assert.equal(result.status, 0, result.stderr + result.stdout);
}
function fails(result) {
  assert.notEqual(result.status, 0, result.stderr + result.stdout);
}

function head(f, branch = "release/0.17.3") {
  return f.git("ls-remote", "origin", `refs/heads/${branch}`).split(/\s+/)[0];
}
function github(f) {
  return JSON.parse(readFileSync(f.env.GH_STATE));
}
function changeGithub(f, update) {
  const state = github(f);
  update(state);
  writeFileSync(f.env.GH_STATE, JSON.stringify(state));
}
function built(f, mode = "rc") {
  succeeds(f.prepare(mode));
  f.build();
  succeeds(f.seal());
}

test("first RC retries the same version and publishes only after building", (t) => {
  const f = fixture(t);
  const refs = f.refs();
  succeeds(f.prepare());
  assert.equal(f.refs(), refs);
  assert.equal(f.git("rev-parse", "HEAD^"), f.start);
  assert.equal(
    JSON.parse(f.git("show", "HEAD:package.json")).version,
    "0.17.3-rc.1"
  );
  assert.equal(f.git("status", "--porcelain"), "");
  assert.equal(f.git("ls-remote", "origin", "refs/tags/*"), "");
  fails(f.seal());
  assert.equal(f.refs(), refs);
  f.git("checkout", "--detach", f.start);
  succeeds(f.prepare("rc", { GITHUB_RUN_ATTEMPT: "2" }));
  const commit = f.git("rev-parse", "HEAD");
  assert.equal(
    JSON.parse(f.git("show", "HEAD:package.json")).version,
    "0.17.3-rc.1"
  );
  f.build();
  succeeds(f.seal());
  assert.equal(f.refs(), refs);
  f.git("checkout", "--detach", f.start);
  succeeds(f.publish());
  assert.equal(head(f), commit);
  assert.equal(head(f, "main"), f.start);
  assert.equal(
    f.git("ls-remote", "origin", "refs/tags/v0.17.3-rc.1^{}").split(/\s+/)[0],
    commit
  );
  assert.deepEqual(
    github(f).prs.map(({ base, head }) => [base, head]),
    [["main", "version/0.17.4-dev"]]
  );
  f.git("fetch", "origin", "refs/heads/version/0.17.4-dev");
  assert.equal(f.git("rev-parse", "FETCH_HEAD^"), f.start);
  assert.equal(
    JSON.parse(f.git("show", "FETCH_HEAD:package.json")).version,
    "0.17.4-dev"
  );
  assert.equal(github(f).releases[0].prerelease, true);
});

test("first patch RC starts directly from a final version on main", (t) => {
  const f = fixture(t, "0.17.2");
  f.git("push", "origin", "HEAD:refs/heads/release/0.17.2");
  const refs = f.refs();
  succeeds(f.prepare());
  assert.equal(f.refs(), refs);
  fails(f.seal());
  assert.equal(f.refs(), refs);
  f.git("checkout", "--detach", f.start);
  succeeds(f.prepare("rc", { GITHUB_RUN_ATTEMPT: "2" }));
  const commit = f.git("rev-parse", "HEAD");
  assert.equal(f.git("rev-parse", "HEAD^"), f.start);
  assert.equal(
    JSON.parse(f.git("show", "HEAD:package.json")).version,
    "0.17.3-rc.1"
  );
  f.build();
  succeeds(f.seal());
  f.git("checkout", "--detach", f.start);
  succeeds(f.publish());
  assert.equal(head(f), commit);
  assert.equal(head(f, "main"), f.start);
  assert.equal(github(f).releases[0].tag_name, "v0.17.3-rc.1");
  const publishedRefs = f.refs();
  const duplicate = f.prepare();
  fails(duplicate);
  assert.match(
    duplicate.stderr,
    /Release branch already exists; select release\/0\.17\.3/
  );
  assert.equal(f.refs(), publishedRefs);
});

test("an RC on main can recreate a missing release branch after building", (t) => {
  const f = fixture(t, "0.17.3-rc.3");
  const dispatch = { GITHUB_REF: "refs/heads/main" };
  f.git("switch", "main");
  const existingRefs = f.refs();
  const conflict = f.prepare("rc", dispatch);
  fails(conflict);
  assert.match(conflict.stderr, /RC is in an inconsistent state/);
  assert.match(conflict.stderr, /This RC must be abandoned/);
  assert.match(
    conflict.stderr,
    /docs\/development\/release_cycle\.md#abandon-an-inconsistent-rc/
  );
  assert.equal(f.refs(), existingRefs);
  assert.equal(f.git("rev-parse", "HEAD"), f.start);
  f.git("push", "origin", "--delete", f.branch);
  const refs = f.refs();
  succeeds(f.prepare("rc", dispatch));
  assert.equal(f.refs(), refs);
  const commit = f.git("rev-parse", "HEAD");
  assert.equal(f.git("rev-parse", "HEAD^"), f.start);
  f.build();
  succeeds(f.seal());
  assert.equal(f.refs(), refs);
  f.git("checkout", "--detach", f.start);
  succeeds(f.publish());
  assert.equal(head(f), commit);
  assert.equal(head(f, "main"), f.start);
  assert.equal(github(f).releases[0].tag_name, "v0.17.3-rc.4");
  assert.deepEqual(github(f).prs, []);
});

test("next RC retries without advancing the release branch before publication", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  const refs = f.refs();
  succeeds(f.prepare());
  fails(f.seal());
  assert.equal(f.refs(), refs);
  f.git("checkout", "--detach", f.start);
  succeeds(f.prepare("rc", { GITHUB_RUN_ATTEMPT: "2" }));
  assert.equal(
    JSON.parse(f.git("show", "HEAD:src-tauri/tauri.conf.json")).version,
    "0.17.3-rc.2"
  );
  f.build();
  succeeds(f.seal());
  assert.equal(f.refs(), refs);
  succeeds(f.publish());
  assert.equal(head(f), f.manifest().commit);
  assert.equal(f.git("ls-remote", "origin", "refs/heads/version/*"), "");
  assert.deepEqual(github(f).prs, []);
});

test("the final publication opens and reuses the release PR after RC publication", (t) => {
  const f = fixture(t);
  built(f);
  succeeds(f.publish());
  assert.deepEqual(
    github(f).prs.map(({ head }) => head),
    ["version/0.17.4-dev"]
  );
  succeeds(f.prepare("release", { GITHUB_REF: "refs/heads/release/0.17.3" }));
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  succeeds(f.publish());
  assert.deepEqual(
    github(f).prs.map(({ base, head }) => [base, head]),
    [
      ["main", "version/0.17.4-dev"],
      ["main", "release/0.17.3"],
    ]
  );
  assert.equal(github(f).merges.length, 0);
});

test("Release commits locally before building and pushes branch and tag only on success", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  const refs = f.refs();
  succeeds(f.prepare("release"));
  const commit = f.git("rev-parse", "HEAD");
  assert.notEqual(commit, f.start);
  assert.equal(
    JSON.parse(f.git("show", "HEAD:package.json")).version,
    "0.17.3"
  );
  assert.equal(f.refs(), refs);
  fails(f.seal());
  f.build();
  fails(f.seal({ SIGNATURE_STATUS: "1" }));
  assert.equal(f.refs(), refs);
  succeeds(f.seal());
  assert.equal(f.refs(), refs);
  succeeds(f.publish());
  assert.equal(head(f), commit);
  assert.equal(head(f, "main"), f.start);
  assert.equal(github(f).releases[0].prerelease, false);
  assert.equal(github(f).prs.length, 1);
});

test("a fresh Release run after failure starts from the same RC", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  const refs = f.refs();
  succeeds(f.prepare("release"));
  fails(f.seal());
  f.git("reset", "--hard", f.start);
  succeeds(f.prepare("release"));
  assert.equal(f.git("rev-parse", "HEAD^"), f.start);
  assert.equal(f.refs(), refs);
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
});

test("main may advance independently while a captured RC builds", (t) => {
  const f = fixture(t);
  built(f);
  const candidate = f.manifest().commit;
  f.git("checkout", "--detach", f.start);
  f.write("future.txt", "later development");
  f.git("add", ".");
  f.git("commit", "-m", "Later work");
  f.git("push", "origin", "HEAD:main");
  const main = head(f, "main");
  succeeds(f.publish());
  assert.equal(head(f), candidate);
  assert.equal(head(f, "main"), main);
  f.git("fetch", "origin", "refs/heads/version/0.17.4-dev");
  assert.equal(f.git("rev-parse", "FETCH_HEAD^"), main);
});

test("first RC still publishes its draft when the bump App token is unavailable", (t) => {
  const f = fixture(t);
  built(f);
  const result = f.publish({ VERSION_BUMP_APP_TOKEN: "" });
  fails(result);
  assert.match(result.stderr, /Version bumps require .*GitHub App token/);
  assert.equal(head(f), f.manifest().commit);
  assert.equal(github(f).releases[0].tag_name, f.manifest().tag);
  assert.deepEqual(github(f).prs, []);
});

test("first RC publication retries bump PR creation using the same refs", (t) => {
  const f = fixture(t);
  built(f);
  fails(f.publish({ GH_FAIL_PR_HEAD: "version/0.17.4-dev" }));
  assert.equal(head(f), f.manifest().commit);
  assert.deepEqual(github(f).prs, []);
  assert.equal(github(f).releases[0].tag_name, f.manifest().tag);
  f.git("checkout", "--detach", f.start);
  f.git("commit", "--allow-empty", "-m", "Advance main during recovery");
  f.git("push", "origin", "HEAD:main");
  const refs = f.refs();
  succeeds(f.publish());
  succeeds(f.publish());
  assert.equal(f.refs(), refs);
  assert.deepEqual(
    github(f).prs.map(({ head }) => head),
    ["version/0.17.4-dev"]
  );
});

for (const [version, mode, ref] of [
  ["0.17.3-dev", "rc", "refs/heads/arbitrary"],
  ["0.17.3-dev", "release", "refs/heads/main"],
  ["0.17.3-rc.1", "release", "refs/heads/main"],
  ["0.17.3-rc.1", "rc", "refs/heads/release/0.17.4"],
  ["0.17.3", "rc", "refs/heads/release/0.17.3"],
  ["0.17.3", "release", "refs/heads/release/0.17.3"],
]) {
  test(`rejects ${mode} of ${version} on ${ref} without changes`, (t) => {
    const f = fixture(t, version);
    const refs = f.refs();
    fails(f.prepare(mode, { GITHUB_REF: ref }));
    assert.equal(f.refs(), refs);
    assert.equal(f.git("status", "--porcelain"), "");
  });
}

test("an existing destination rejects first RC without overwriting it", (t) => {
  const f = fixture(t);
  f.git("push", "origin", "HEAD:release/0.17.3");
  const refs = f.refs();
  fails(f.prepare());
  assert.equal(f.refs(), refs);
});

test("dirty source and source changes during building are rejected", (t) => {
  const f = fixture(t);
  f.write("untracked", "dirty");
  fails(f.prepare());
  rmSync(join(f.root, "untracked"));
  succeeds(f.prepare());
  f.build();
  f.write("application.txt", "changed");
  fails(f.seal());
  f.git("add", "application.txt");
  fails(f.seal());
});

test("release preparation rejects the retired tag mode", (t) => {
  const f = fixture(t, "0.17.3");
  f.git("tag", "-a", "v0.17.3", "-m", "Release");
  f.git("push", "origin", "v0.17.3");
  const refs = f.refs();
  const result = f.prepare("tag", { GITHUB_REF: "refs/tags/v0.17.3" });
  fails(result);
  assert.match(result.stderr, /Expected rc or release mode/);
  assert.equal(f.refs(), refs);
});

for (const failure of ["upload", "pr-create"]) {
  test(`publication recovers from ${failure} using the same package`, (t) => {
    const f = fixture(t, "0.17.3-rc.1");
    built(f, "release");
    fails(f.publish({ GH_FAIL: failure }));
    const refs = f.refs();
    succeeds(f.publish());
    succeeds(f.publish());
    assert.equal(f.refs(), refs);
    assert.equal(github(f).prs.length, 1);
  });
}

test("publication recovers in a clean original-source checkout using the retained package", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  built(f, "release");
  const commit = f.manifest().commit;
  f.git("reset", "--hard", f.start);
  succeeds(f.publish());
  assert.equal(head(f), commit);
});

for (const failure of [
  "corruption",
  "tag conflict",
  "branch moved",
  "tag rejected",
]) {
  test(`Release ${failure} cannot partially update branch and tag`, (t) => {
    const f = fixture(t, "0.17.3-rc.1");
    built(f, "release");
    if (failure === "corruption")
      writeFileSync(
        join(f.env.RELEASE_PACKAGE, f.manifest().assets[0].name),
        "corrupted"
      );
    if (failure === "tag conflict")
      f.git("push", "origin", `${f.start}:refs/tags/v0.17.3`);
    if (failure === "branch moved") {
      f.git("checkout", "--detach", f.start);
      f.write("other", "change");
      f.git("add", ".");
      f.git("commit", "-m", "Concurrent change");
      f.git("push", "origin", "HEAD:release/0.17.3");
    }
    if (failure === "tag rejected")
      writeFileSync(
        join(f.remote, "hooks/pre-receive"),
        '#!/bin/sh\nwhile read old new ref; do\ncase "$ref" in refs/tags/*) exit 1;; esac\ndone\n',
        { mode: 0o755 }
      );
    const refs = f.refs();
    fails(f.publish());
    assert.equal(f.refs(), refs);
  });
}

test("publication refuses to overwrite published drafts", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  built(f, "release");
  succeeds(f.publish());
  changeGithub(f, (s) => {
    s.releases[0].draft = false;
  });
  const uploads = github(f).uploads.length;
  fails(f.publish());
  assert.equal(github(f).uploads.length, uploads);
});

test("a concurrent branch push during final publication rejects the atomic update", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  built(f, "release");
  const other = f.git(
    "commit-tree",
    f.git("rev-parse", `${f.start}^{tree}`),
    "-p",
    f.start,
    "-m",
    "Concurrent fix"
  );
  f.git("--git-dir", f.remote, "fetch", f.root, other);
  const quote = (value) => "'" + value.replaceAll("'", "'\\''") + "'";
  writeFileSync(
    join(f.root, ".git/hooks/pre-push"),
    `#!/bin/sh\ngit --git-dir=${quote(f.remote)} update-ref refs/heads/release/0.17.3 ${other} ${f.start}\n`,
    { mode: 0o755 }
  );
  fails(f.publish());
  assert.equal(head(f), other);
  assert.equal(f.git("ls-remote", "origin", "refs/tags/*"), "");
});

test("RC status starts at publication on the prepared commit", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  succeeds(f.status("failure"));
  f.build();
  succeeds(f.seal());
  const commit = f.manifest().commit;
  succeeds(f.publish());
  assert.deepEqual(github(f).statuses, [
    { sha: commit, state: "pending", context: "Release build" },
    { sha: commit, state: "success", context: "Release build" },
  ]);
});

test("a local final commit does not alter its successful RC status", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  succeeds(f.prepare("release"));
  succeeds(f.status("failure"));
  assert.equal(head(f), f.start);
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  assert.deepEqual(
    github(f).statuses.map((status) => status.state),
    ["pending", "success"]
  );
  assert.ok(
    github(f).statuses.every((status) => status.sha === f.manifest().commit)
  );
});

test("failed publication cannot report success and recovery reports success on the same commit", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  built(f, "release");
  fails(f.publish({ GH_FAIL: "pr-create" }));
  succeeds(f.status("failure", join(f.env.RELEASE_PACKAGE, "release.json")));
  assert.equal(github(f).statuses.at(-1).state, "failure");
  assert.ok(github(f).statuses.every((status) => status.state !== "success"));
  succeeds(f.publish());
  assert.equal(github(f).statuses.at(-1).state, "success");
  assert.ok(
    github(f).statuses.every((status) => status.sha === f.manifest().commit)
  );
});

test("failure reporting without a prepared commit is a no-op", (t) => {
  const f = fixture(t);
  succeeds(f.status("failure", join(f.directory, "missing-state.json")));
  assert.equal(head(f, "main"), f.start);
});
