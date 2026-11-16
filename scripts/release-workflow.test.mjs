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
    "release-evidence.mjs",
    "verify-release-pr.mjs",
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
  git("commit", "--allow-empty", "-m", "Base");
  git("add", ".");
  git("commit", "-m", "Initial source");
  git("init", "--bare", "--initial-branch=main", remote);
  git("remote", "add", "origin", remote);
  git("push", "origin", "main");
  const start = git("rev-parse", "HEAD");
  const branch = version.endsWith("-dev")
    ? "develop"
    : `release/${version.replace(/-rc\.[0-9]+$/, "")}`;
  git("switch", "-c", branch);
  git("push", "origin", branch);
  const quote = (value) => "'" + value.replaceAll("'", "'\\''") + "'";
  const mockPath = join(directory, "gh.cjs");
  writeFileSync(
    mockPath,
    `
const fs = require('node:fs');
const path = process.env.GH_STATE;
const state = fs.existsSync(path) ? JSON.parse(fs.readFileSync(path)) : { releases: [], uploads: [], records: {}, runs: {}, prs: [], merges: [] };
const args = process.argv.slice(2);
if (process.env.GH_FAIL === args[1] || process.env.GH_FAIL === args.slice(0,2).join('-')) process.exit(17);
if (args[0] === 'api' && args[1] === 'graphql') console.log(JSON.stringify({data:{repository:{autoMergeAllowed:true,mergeCommitAllowed:true,deleteBranchOnMerge:false,ref:{branchProtectionRule:state.protection || {requiresStatusChecks:true, requiresStrictStatusChecks:true, requiredStatusCheckContexts:['Release PR verification']}}}}}));
else if (args[0] === 'api' && args[1].includes('/actions/runs/')) {
 const id=args[1].split('/runs/')[1].split('/')[0];
 if (!state.runs[id]) process.exit(18);
 console.log(JSON.stringify(state.runs[id]));
}
else if (args[0] === 'api' && args[1].includes('/statuses/')) {
 state.statuses ||= []; state.statuses.push(args.filter(a=>a.startsWith('state='))[0].slice(6));
}
else if (args[0] === 'api' && args[1].includes('/pulls/')) console.log(JSON.stringify(state.prDetails));
else if (args[0] === 'api') console.log(JSON.stringify([state.releases]));
else if (args[0] === 'pr' && args[1] === 'list') console.log(JSON.stringify(state.prs));
else if (args[0] === 'pr' && args[1] === 'create') state.prs.push({number:1});
else if (args[0] === 'pr' && args[1] === 'merge') state.merges.push(args);
else if (args[1] === 'download') {
 if (!state.records[args[2]]) process.exit(18);
 console.log(JSON.stringify(state.records[args[2]]));
}
else if (args[1] === 'create') state.releases.push({tag_name: args[2], draft: true, prerelease: args.includes('--prerelease')});
else if (args[1] === 'upload') {
 state.uploads.push(args[2]);
 const record = JSON.parse(fs.readFileSync(args.find(a=>a.endsWith('/release.json'))));
 state.records[args[2]]=record;
 state.runs[record.runId]={run_attempt:Number(record.runAttempt),conclusion:'success',status:'completed',event:'workflow_dispatch',path:record.mode==='rc'?'.github/workflows/prepare-rc.yml':'.github/workflows/prepare-release.yml',head_sha:record.start,head_branch:record.sourceRef.replace('refs/heads/',''),repository:{full_name:'fixture/xaga'}};
}
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
    GITHUB_REF: `refs/heads/${branch}`,
    GITHUB_RUN_ID: "100",
    GITHUB_RUN_ATTEMPT: "1",
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
  const seedEvidence = () => {
    const version = JSON.parse(
      readFileSync(join(root, "src-tauri/tauri.conf.json"))
    ).version;
    const tag = `v${version}`;
    git("tag", "-a", tag, "-m", "Verified candidate");
    git("push", "origin", tag);
    const parent = git("rev-parse", "HEAD^");
    const record = {
      mode: "rc",
      version,
      tag,
      commit: start,
      branch: `refs/heads/${branch}`,
      sourceRef: `refs/heads/${branch}`,
      start: parent,
      tree: git("rev-parse", "HEAD^{tree}"),
      tagObject: git("rev-parse", `refs/tags/${tag}`),
      runId: "90",
      runAttempt: "1",
    };
    writeFileSync(
      env.GH_STATE,
      JSON.stringify({
        releases: [],
        uploads: [],
        prs: [],
        merges: [],
        records: { [tag]: record },
        runs: {
          90: {
            run_attempt: 1,
            conclusion: "success",
            status: "completed",
            event: "workflow_dispatch",
            path: ".github/workflows/prepare-rc.yml",
            head_sha: parent,
            head_branch: branch,
            repository: { full_name: "fixture/xaga" },
          },
        },
      })
    );
  };
  return {
    root,
    branch,
    seedEvidence,
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

  ["0.17.3-rc.2", "release", "0.17.3"],
]) {
  test(`${mode} prepares ${before} as ${after} without committing or pushing`, (t) => {
    const f = fixture(t, before);
    if (mode === "release") f.seedEvidence();
    const tags = f.git("tag", "--list");
    const refs = f.refs();
    succeeds(f.prepare(mode));
    assert.equal(JSON.parse(readFileSync(f.env.RELEASE_STATE)).version, after);
    assert.equal(f.git("rev-parse", "HEAD"), f.start);
    assert.equal(f.git("tag", "--list"), tags);
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
  f.seedEvidence();
  succeeds(f.prepare("release"));
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  const gh = JSON.parse(readFileSync(f.env.GH_STATE));
  assert.equal(gh.releases[0].draft, true);
  assert.equal(gh.releases[0].prerelease, false);
});

test("release branch movement rejects both updates", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  f.git("checkout", "--detach", f.start);
  f.write("application.txt", "intervening change");
  f.git("add", "application.txt");
  f.git("commit", "-m", "Intervening change");
  f.git("push", "origin", `HEAD:${f.branch}`);
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
  const f = fixture(t, "0.17.3-rc.1");
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
    `#!/bin/sh\ngit --git-dir=${quote(f.remote)} update-ref refs/heads/${f.branch} ${other} ${f.start}\n`,
    { mode: 0o755 }
  );
  fails(f.publish());
  assert.equal(
    f.git("ls-remote", "origin", `refs/heads/${f.branch}`),
    `${other}\trefs/heads/${f.branch}`
  );
  assert.equal(f.git("ls-remote", "origin", "refs/tags/*"), "");
});

test("competing RC and final preparation cannot both update the same starting branch", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  f.seedEvidence();
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

test("arbitrary branches cannot prepare candidates or final releases", (t) => {
  const f = fixture(t);
  f.git("switch", "-c", "emergency/patch");
  f.git("push", "origin", "emergency/patch");
  fails(f.prepare("rc", { GITHUB_REF: "refs/heads/emergency/patch" }));
  fails(f.prepare("release", { GITHUB_REF: "refs/heads/develop" }));
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

function updateGithub(f, update) {
  const state = JSON.parse(readFileSync(f.env.GH_STATE));
  update(state);
  writeFileSync(f.env.GH_STATE, JSON.stringify(state));
}

test("first RC creates a release branch without updating develop or main or opening a PR", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  const state = f.manifest();
  assert.equal(state.branch, "refs/heads/release/0.17.3");
  for (const branch of ["main", "develop"])
    assert.equal(
      f.git("ls-remote", "origin", `refs/heads/${branch}`),
      `${f.start}\trefs/heads/${branch}`
    );
  assert.deepEqual(JSON.parse(readFileSync(f.env.GH_STATE)).prs, []);
  f.git("reset", "--hard", f.start);
  fails(f.prepare());
});

test("develop can advance while its captured candidate builds", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  const candidate = f.git("rev-parse", "HEAD");
  f.git("checkout", "--detach", f.start);
  f.write("application.txt", "new development");
  f.git("add", "application.txt");
  f.git("commit", "-m", "More work");
  f.git("push", "origin", "HEAD:develop");
  succeeds(f.publish());
  assert.equal(f.manifest().commit, candidate);
});

test("a competing first run cannot replace an existing release branch", (t) => {
  const f = fixture(t);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  f.git("push", "origin", `${f.start}:refs/heads/release/0.17.3`);
  const refs = f.refs();
  fails(f.publish());
  assert.equal(f.refs(), refs);
});

test("subsequent RCs do not import newer develop commits", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  f.git("switch", "-c", "develop");
  f.write("application.txt", "future work");
  f.git("add", "application.txt");
  f.git("commit", "-m", "Future work");
  f.git("push", "origin", "develop");
  f.git("switch", f.branch);
  succeeds(f.prepare());
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  assert.equal(f.git("show", "HEAD:application.txt"), "original source");
});

test("Release refuses an RC without successful workflow evidence", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  fails(f.prepare("release"));
  assert.equal(f.git("status", "--porcelain"), "");
  f.seedEvidence();
  updateGithub(f, (s) => {
    s.runs[90].conclusion = "failure";
  });
  fails(f.prepare("release"));
  assert.equal(f.git("status", "--porcelain"), "");
});

test("Release refuses fixes added after a verified RC", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  f.seedEvidence();
  f.write("application.txt", "release fix");
  f.git("add", "application.txt");
  f.git("commit", "-m", "Fix");
  f.git("push", "origin", `HEAD:${f.branch}`);
  fails(f.prepare("release"));
  assert.equal(f.git("status", "--porcelain"), "");
  succeeds(f.prepare("rc"));
});

test("Release rejects mismatched evidence from another workflow or source tree", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  f.seedEvidence();
  updateGithub(f, (s) => {
    s.runs[90].path = ".github/workflows/release.yml";
  });
  fails(f.prepare("release"));
  updateGithub(f, (s) => {
    s.runs[90].path = ".github/workflows/prepare-rc.yml";
    s.records["v0.17.3-rc.1"].tree = "a".repeat(40);
  });
  fails(f.prepare("release"));
});

test("final publication requires configured protection before pushing", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  f.seedEvidence();
  succeeds(f.prepare("release"));
  f.build();
  succeeds(f.seal());
  updateGithub(f, (s) => {
    s.protection = {
      requiresStatusChecks: true,
      requiresStrictStatusChecks: false,
      requiredStatusCheckContexts: [],
    };
  });
  const refs = f.refs();
  fails(f.publish());
  assert.equal(f.refs(), refs);
});

test("final publication opens a PR and requests auto-merge for the exact built commit", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  f.seedEvidence();
  succeeds(f.prepare("release"));
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  let state = JSON.parse(readFileSync(f.env.GH_STATE));
  assert.equal(state.prs.length, 1);
  assert.ok(state.merges[0].includes("--auto"));
  assert.ok(state.merges[0].includes(f.manifest().commit));
  assert.ok(!state.merges[0].includes("--delete-branch"));
  succeeds(f.publish());
  state = JSON.parse(readFileSync(f.env.GH_STATE));
  assert.equal(state.prs.length, 1);
  assert.equal(
    f.git("ls-remote", "origin", "refs/heads/main"),
    `${f.start}\trefs/heads/main`
  );
});

function finalPr(t) {
  const f = fixture(t, "0.17.3-rc.1");
  f.seedEvidence();
  succeeds(f.prepare("release"));
  f.build();
  succeeds(f.seal());
  succeeds(f.publish());
  const head = f.manifest().commit;
  f.git("push", "origin", `${head}:refs/pull/1/head`);
  updateGithub(f, (s) => {
    s.prDetails = {
      state: "open",
      head: { sha: head, ref: f.branch, repo: { full_name: "fixture/xaga" } },
      base: { ref: "main", sha: f.start },
    };
  });
  const event = join(f.directory, "event.json");
  writeFileSync(event, JSON.stringify({ number: 1 }));
  const verify = () =>
    spawnSync(
      process.execPath,
      [join(f.root, "scripts/verify-release-pr.mjs")],
      {
        cwd: f.root,
        env: {
          ...f.env,
          GITHUB_EVENT_NAME: "pull_request_target",
          GITHUB_EVENT_PATH: event,
        },
        encoding: "utf8",
      }
    );
  return { ...f, verify, event };
}

test("merge verification accepts the identical final source without another build", (t) => {
  const f = finalPr(t);
  succeeds(f.verify());
  assert.equal(
    JSON.parse(readFileSync(f.env.GH_STATE)).statuses.at(-1),
    "success"
  );
});

test("merge verification rejects untested changes introduced by main", (t) => {
  const f = finalPr(t);
  f.git("checkout", "--detach", f.start);
  f.write("main-only.txt", "new main change");
  f.git("add", "main-only.txt");
  f.git("commit", "-m", "Main changed");
  f.git("push", "origin", "HEAD:main");
  const base = f.git("rev-parse", "HEAD");
  updateGithub(f, (s) => {
    s.prDetails.base.sha = base;
  });
  fails(f.verify());
  assert.equal(
    JSON.parse(readFileSync(f.env.GH_STATE)).statuses.at(-1),
    "failure"
  );
});

test("merge verification refuses a pending or failed final workflow", (t) => {
  const f = finalPr(t);
  updateGithub(f, (s) => {
    s.runs[100].conclusion = null;
    s.runs[100].status = "in_progress";
  });
  fails(f.verify());
  updateGithub(f, (s) => {
    s.runs[100].conclusion = "success";
    s.runs[100].status = "completed";
  });
  succeeds(f.verify());
});

test("final PR creation failure recovers without changing refs or duplicating PRs", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  f.seedEvidence();
  succeeds(f.prepare("release"));
  f.build();
  succeeds(f.seal());
  fails(f.publish({ GH_FAIL: "pr-create" }));
  const refs = f.refs();
  succeeds(f.publish());
  assert.equal(f.refs(), refs);
  assert.equal(JSON.parse(readFileSync(f.env.GH_STATE)).prs.length, 1);
});

test("Release completion verifies the PR even without a PR event", (t) => {
  const f = finalPr(t);
  writeFileSync(
    f.event,
    JSON.stringify({ workflow_run: { head_branch: f.branch } })
  );
  const result = spawnSync(
    process.execPath,
    [join(f.root, "scripts/verify-release-pr.mjs")],
    {
      cwd: f.root,
      env: {
        ...f.env,
        GITHUB_EVENT_NAME: "workflow_run",
        GITHUB_EVENT_PATH: f.event,
      },
      encoding: "utf8",
    }
  );
  succeeds(result);
  assert.equal(
    JSON.parse(readFileSync(f.env.GH_STATE)).statuses.at(-1),
    "success"
  );
});

test("an RC pull request cannot pass the main merge gate", (t) => {
  const f = finalPr(t);
  f.git("push", "--force", "origin", `${f.start}:refs/pull/1/head`);
  updateGithub(f, (s) => {
    s.prDetails.head.sha = f.start;
  });
  fails(f.verify());
  assert.equal(
    JSON.parse(readFileSync(f.env.GH_STATE)).statuses.at(-1),
    "failure"
  );
});

test("Release accepts a successful publication retry of the original RC build", (t) => {
  const f = fixture(t, "0.17.3-rc.1");
  f.seedEvidence();
  updateGithub(f, (s) => {
    s.runs[90].run_attempt = 2;
    s.runs[90].conclusion = "failure";
  });
  fails(f.prepare("release"));
  updateGithub(f, (s) => {
    s.runs[90].conclusion = "success";
  });
  succeeds(f.prepare("release"));
});
