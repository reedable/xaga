# Development, Build, and Release Cycle

## Version lifecycle

The committed application version describes its lifecycle state:

| State             | Version                   | Meaning                                                                                    |
| ----------------- | ------------------------- | ------------------------------------------------------------------------------------------ |
| Development       | `0.8.5-dev`               | Work toward the next release is in progress.                                               |
| Local alpha       | `0.8.5-dev+alpha`         | An invocation-only version identifies an ordinary local build, including work in progress. |
| Local beta        | `0.8.5-rc.1+beta.9c62ffc` | An invocation-only version identifies local validation of an untagged RC or final version. |
| Release candidate | `0.8.5-rc.1`              | A distributable candidate for the final `0.8.5` release.                                   |
| Final             | `0.8.5`                   | The published release version.                                                             |

The normal ordering is:

```text
0.8.4
0.8.5-dev
0.8.5-rc.1
0.8.5-rc.1+beta.<commit>
0.8.5-rc.2
0.8.5
0.8.6-dev
```

`-dev` and `-rc.N` are SemVer prerelease identifiers. `+alpha` and `+beta.<commit>` are build metadata. The alpha suffix identifies an ordinary local build; the beta suffix identifies local validation of an RC without creating another release stage. Alpha and beta versions are local-only and must not be Git tags or GitHub Releases.

`src-tauri/tauri.conf.json` is the application-version source of truth. `scripts/bump-version.mjs` keeps it synchronized with `package.json` and the `[package]` versions in:

- `src-tauri/Cargo.toml`
- `crates/xaga-core/Cargo.toml`
- `crates/xaga-cli/Cargo.toml`

The version commands update the three Xaga workspace package entries in `Cargo.lock` directly alongside the source version files. They do not invoke Cargo, resolve dependencies, access the network, or compile the Rust workspace.

While the major version is 0, minor versions signal significant compatibility
changes. Every library storage-model change requires a minor-version increment.
The 0.10 reading-state format requires the [explicit library upgrade](../library_upgrade.md)
before using a 0.9 library; patch versions do not carry this format transition.
The 0.13 article-color format likewise requires an explicit upgrade and minor
version increment; it removes the old bookmark boolean and tag representation.
The 0.14 tag registry and indexed UUID assignments require the matching CLI
upgrade; rename, merge, and deletion storage is included in that format.
The 0.15 nested pin-group format requires an explicit upgrade; flat pin files
retain their encoding and order.

Library versions use the application's `major.minor` convention. Each library
stores its own completed version in `library.yaml`; it is read at runtime and is
advanced only by an explicit successful upgrade. Patch and prerelease changes do
not alter the library version. Changing the app version does not relabel existing
libraries. See [library version metadata](../architecture/library_version.md).

## Starting development

After publishing a final release, select the kind of next release and enter development immediately:

```sh
npm run version:patch
npm run version:minor
npm run version:major
```

Only one command is used. For example, `patch` changes `0.8.4` to `0.8.5-dev`; `minor` changes it to `0.9.0-dev`; and `major` changes it to `1.0.0-dev`.

When entering `x.y.0-dev`, follow the
[maintenance review trigger](../maintenance/index.md#review-trigger) and remind
the user to select recurring maintenance work for the new development cycle.

Commit the synchronized version files and refreshed `Cargo.lock` as the start of the new development cycle. A major, minor, or patch command may retarget an existing `-dev` version. It is rejected for an `-rc.N` version because a release candidate must be finalized before the next cycle begins.

## Development and local builds

Use the normal development command while changing the application:

```sh
npm run dev
```

The short `dev` and `build` commands operate on the Xaga desktop application. Vite-only commands live under the `ui:` namespace:

```sh
npm run ui:dev
npm run ui:build
npm run ui:preview
```

Run all tests with `npm test`: script and UI tests, Rust unit and ACL integration
tests, Rust documentation tests, Playwright browser tests, and the native WebKit suite. The command stops
on the first failure. The full suite requires macOS with a logged-in desktop
session; the fingerprint probe opens a local page in Safari. Native download
tests run every supported scenario, and the SVG wrapper builds its fixture.

Install Chromium once with `npx playwright install chromium` before running browser
tests. Use `npm run test:ci` for all suites that do not require a desktop session.
Use `npm run test:headless` for script, UI, and Rust tests without Playwright, or
`npm run test:gui` for native WebKit tests only. Individual suites remain
available as `npm run test:scripts`, `npm run test:ui`, `npm run test:rust`, and
`npm run test:browser`.
Pass Vitest file filters through `npm run test:ui -- <files>`.

The ordinary production build is:

```sh
npm run build
```

It runs the advisory Rust dependency audit and then `npm run build:alpha`. The audit report does not stop the build; compilation or bundling failures do.

Alpha builds add exactly `+alpha` to the configured application version for this invocation, such as `0.14.3-dev+alpha`, `0.14.3-rc.1+alpha`, or `0.14.3+alpha`. They allow a dirty working tree and tagged versions, require no Git commit ID, and do not edit version files or lockfiles. Running `build:alpha` directly skips the advisory audit.

Both alpha and beta builds use the Alpha application icon, as does `npm run dev`. Automated releases retain the standard icon. Additional Tauri build flags remain available, for example:

```sh
npm run build:alpha -- --no-bundle
```

Rust compilation requires the Protocol Buffers compiler because Lance generates
code during its build. On macOS, install it with `brew install protobuf` and
verify it is available with `protoc --version`. The shared release pipeline installs
this prerequisite before verification and building.

## Prepare releases with GitHub Actions

All development targets `main` through pull requests. Release workflows write
only to `release/x.y.z` and release tags; they do not push to `main`, merge PRs,
or inspect administrative branch-protection settings.

### Release Candidate

Run **Release Candidate** (`.github/workflows/prepare-rc.yml`) on `main` at
`x.y.z-dev` to immediately commit `x.y.z-rc.1` and create `release/x.y.z` on the
remote, before verification and building. If the release branch already exists,
select that branch instead. On `release/x.y.z` at `x.y.z-rc.N`, the workflow
commits and pushes the next RC version before verification and building.

A successful build creates the RC tag, draft prerelease, and a PR from the release
branch to `main`, reusing an existing open PR. A failed build leaves the RC commit
on the release branch without a tag or a new PR. An existing PR updates as soon
as the RC commit is pushed, even if that build later fails.

### Release

Run **Release** (`.github/workflows/prepare-release.yml`) only on
`release/x.y.z` at an RC version. It removes the RC suffix and commits `x.y.z`
locally before verification and building. After success, it atomically pushes
that commit and the final tag, updating the release branch's PR, and uploads a
final draft release. A failure before the atomic push leaves the remote branch
and PR on the RC with no final tag. A fresh run starts from that same remote RC.
No separate RC build-record or workflow-success gate is required: Release checks
and builds its own final commit.

Merging the PR and publishing the draft remain manual. Required reviews and
checks on `main` still apply. With a merge commit, the tagged release commits
become part of `main`'s history. Ongoing development may advance `main` while a
release builds; resolve any merge conflicts through the normal PR process.

### Workflow setup

Both the default branch and selected branch must contain the workflow and its
supporting scripts. The workflows use `GITHUB_TOKEN` with `contents: write` and
`pull-requests: write` and `statuses: write`; enable **Allow GitHub Actions to create and approve pull
requests** in repository Actions settings. The workflows create PRs but do not
approve them. Branch and tag rules must permit the bot to create/update
`release/*` and create release tags. Protect `main` by requiring PRs. No bot
bypass of `main`, auto-merge setting, or administrative-read credential is needed.

### Required release status

The workflows report a commit status named **Release build** on the prepared
RC or final commit, rather than on the commit selected for workflow dispatch.
RC preparation reports pending after pushing the version commit. A final commit
remains local during building, so its status starts after the atomic push.
Success is reported only after build, artifact retention, draft upload, and PR
creation or reuse succeed. Failure/cancellation cleanup reports failure when the
prepared commit is available remotely. If the runner cannot execute cleanup,
a missing or pending required status continues to block merging. Publication
retries report pending and then success on the same prepared commit.

To require this status for `main`:

1. Install these workflows on the default and selected source branches, then run
   Release Candidate or Release successfully once to register **Release build**.
2. Open repository **Settings → Branches** and click **Edit** beside the branch
   protection rule for `main`.
3. Enable **Require status checks to pass before merging**. In the search field
   beneath that option, enter `Release build` and select the matching result.
4. Click **Save changes**. GitHub then requires this status on each PR's latest
   commit; an earlier RC's success does not satisfy a newer RC or final commit.

The workflows do not edit repository protection settings. GitHub applies this
required status to all PRs into `main`. The development-PR workflow automatically
reports **Release build** as successful with the description “Release build not
required for development PR” for ordinary PRs. Their other required checks and
reviews still apply. PRs from `release/*` receive no exemption and require the
actual release workflow result. An ordinary PR pointing at the same commit as a
release branch cannot overwrite that commit's release-build result.

The exemption workflow runs on PR creation, updates, reopening, and edits. It
executes only trusted base-branch code and reads PR metadata; it does not check
out or execute PR code, including code from forks. It needs `contents: read` to
identify release-branch heads and `statuses: write` to report the exemption.
Install this workflow on `main` before requiring **Release build**. For an
already-open ordinary PR, push another commit or reopen it to trigger reporting.

## Prepare releases locally

Prepare and commit the release version locally, then push its annotated tag to
trigger **Manual release build**. The workflow builds the tagged commit while
other development can continue. It does not create branches, advance versions,
merge branches, or open pull requests.

### Release candidates

Run `npm run version:rc` from a clean working tree, review and commit the
synchronized version files and `Cargo.lock`, then run `npm run build:beta` for
local validation before tagging. The version command advances development to
RC 1 or increments an existing RC; it never commits, tags, pushes, or builds.

### Finalizing a release

Run `npm run version:release` from a clean working tree to remove `-dev` or
`-rc.N`. It rejects an already-final version. Review and commit the synchronized
version files and `Cargo.lock` before tagging.

### Local beta builds

`npm run build:beta` validates an untagged, committed RC or final version locally.
It uses the Alpha icon and invocation-only `+beta.<commit>` metadata. It requires
an entirely clean tree, rejects development versions, and refuses an existing
matching local release tag. It does not fetch remote tags, edit version files,
commit, tag, push, or publish. Published tags remain immutable.

### Tagging a release

Run `npm run release:tag` from the clean committed tree, then push the commit
and annotated tag with `git push origin <branch> --follow-tags`. The command
checks all source versions and Xaga lockfile entries, rejects development
versions, and refuses an existing local tag. It never commits or pushes.

### GitHub release automation

**Manual release build** (`.github/workflows/release.yml`) starts automatically
when someone pushes a final `vX.Y.Z` or RC `vX.Y.Z-rc.N` tag. It is a tag-push
trigger, not a Run workflow control. The tag must match the checked-out
application version; development tags are rejected.

It calls `.github/workflows/release-pipeline.yml` to run lint, formatting,
TypeScript, CSP, Rust formatting and all-target workspace checks, headless tests,
and browser tests. Each test step has a 30-minute timeout. It then builds the
Apple Silicon distributable with the committed standard icons, verifies its
ad-hoc signature, and retains the artifacts, checksums, and Git identity in a
workflow package for 14 days. The publication job uploads these to a draft
GitHub release; RC drafts are prereleases. Inspect the artifacts before manually
publishing the draft. Native WebKit and bundled macOS smoke checks remain
separate desktop verification; see [verification](verification.md) and
[resource verification](resource_verification.md).

Manual release build uses `GITHUB_TOKEN` with write access to release contents.
It does not need automatic PR creation or auto-merge. A tag pushed by another workflow
using `GITHUB_TOKEN` does not trigger the tag-push workflow.

The former **Release PR verification** status is no longer produced. If it is
still configured as a required check in GitHub branch protection or a ruleset,
remove that required check so it does not block PRs indefinitely. Removing the
workflow file does not update GitHub repository settings.

### Failures and recovery

Failed verification, build, or signature checking does not change the existing
tag. Do not move a release tag to reuse its number after changing source.

For failed draft uploads or PR creation, choose **Re-run failed jobs** on the original run, or
rerun only **Push and create draft**. It reuses the successful build job's
retained package without another build or version bump and refuses to alter a
published release. For automated RC/Release runs, it also reuses the existing
open PR. After a successful final push, recover publication using that retained
package rather than starting Release again on the now-final branch.
Do not use **Re-run all jobs** for draft recovery. Packages and browser diagnostics
expire after 14 days; save the package first if recovery must be delayed.

`node scripts/publish-release.mjs` can recover a saved package from the original
source checkout with `RELEASE_PACKAGE` set to its absolute directory and
`GITHUB_REPOSITORY` set to `owner/repository`, using authenticated Git and GitHub
CLI access. Without the package, exact-artifact recovery is unavailable.

Local tests use isolated repositories and simulated GitHub responses. They do
not establish live GitHub dispatch/tag triggering, repository permissions, PR
creation, draft uploads, or failed-job recovery. Verify these against the
repository settings when installing the workflows.

## Distribution boundary

The automated build applies a local ad-hoc signature and verifies the completed application bundle. That signature supplies bundle integrity but no developer identity: GitHub holds no signing key, and Gatekeeper still requires the user to approve the application manually. Public distribution without that override requires a paid Apple Developer ID certificate and Apple notarization; those credentials are not currently part of the workflow.
