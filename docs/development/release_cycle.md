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

`develop` is the integration branch and accepts direct development pushes.
`release/x.y.z` holds the fixes and candidates for one release. Protected `main`
receives final releases through pull requests; RCs are never merged there.
Keep release branches after merging. Bring release fixes back into `develop`
through the normal development process; these workflows do not synchronize them.

Use **RC** or **Release** in Actions, then **Run workflow** on the website or in
GitHub Mobile. The branch picker selects the source; tag refs and arbitrary
branches are rejected. Both the default branch and selected source branch must
contain the workflow and supporting scripts.

| Workflow | Selected branch | Starting version            | Result                                  |
| -------- | --------------- | --------------------------- | --------------------------------------- |
| RC       | `develop`       | `x.y.z-dev`                 | Create `release/x.y.z` and `x.y.z-rc.1` |
| RC       | `release/x.y.z` | `x.y.z-dev` or `x.y.z-rc.N` | First or next RC on that branch         |
| Release  | `release/x.y.z` | Verified `x.y.z-rc.N`       | Final `x.y.z` and PR to `main`          |

The first RC captures the selected `develop` commit. New development arriving
while it builds is left for another release. If `release/x.y.z` already exists,
RC on `develop` refuses to run: select the release branch instead. Subsequent
release fixes go to that branch, never implicitly import changes from `develop`.
For an emergency release, prepare a matching `release/x.y.z` branch and
`x.y.z-dev` version from the appropriate released source, then run RC there.

Release requires its branch HEAD to match the RC tag and the build record
attached as `release.json` to the candidate's GitHub prerelease. It checks the
record's commit, tree, tag, parent, source branch, and successful RC workflow
attempt in the same repository. A manually created tag or an extra fix commit
is not sufficient: run RC again before finalization. Existing RCs created before
this evidence was recorded likewise need a new RC run. The local version command
still supports development-to-final preparation; the automated Release workflow
does not.

### Verification, build, and commit order

All three build entry points share `.github/workflows/release-pipeline.yml`:

1. Start with a clean checkout, validate the selected branch and lifecycle, and
   stage synchronized version changes without committing.
2. Run lint, formatting, TypeScript, CSP, Rust formatting and all-target workspace
   checks, then headless and browser tests. Each test step has a 30-minute timeout.
3. Build the Apple Silicon distributable with its plain RC or final version and
   committed standard icons, and verify its ad-hoc signature. Source files must
   still match the prepared tree. Local beta builds are not a prerequisite.
4. Commit and annotate the release tag after success. Retain the already-built
   artifacts, checksums, and Git identity in a workflow package for 14 days.
5. Atomically push the release branch and tag. For branch creation, both refs
   must be absent; for an existing branch, its HEAD must still equal the starting
   commit and the tag must be absent. Never push version changes to `main` or
   `develop`.
6. Upload the artifacts and build record to a draft release. RC drafts are
   prereleases and have no PR to `main`. Final preparation creates or reuses a
   release-branch PR to `main` and requests auto-merge for the exact built commit.

The final tag stays on the release-branch commit. The **Verify release PR**
workflow checks that the proposed merge tree equals the verified final build's
source tree, without recompiling. It runs from trusted `main` code and never
executes the candidate's code. It checks PR events and completion of the Release
workflow; the **Release PR verification** status cannot pass while the matching
final workflow is incomplete or unsuccessful. Other required reviews/checks
still apply. Use merge commits to preserve the retained release branch's ancestry.

If `main` moves and changes the proposed merge content, the release must be
reconciled and verified again before merging. Do not edit or move a final tag to
reuse its number. Publish the final draft manually after the PR has merged and
its artifacts have been inspected. Native WebKit and bundled macOS smoke checks
remain separate desktop verification; see [verification](verification.md) and
[resource verification](resource_verification.md).

### Repository setup

Install the workflows and verification scripts on `main` before using the new
release process, and keep `main` as the default branch. Create `develop` for
ongoing work. To adopt this flow with an existing RC on `main`, create its matching
release branch from that RC and run RC there to establish fresh build evidence.
Choose the next development version separately; the workflows do not advance it.

No personal token or separate GitHub App credential is required. `GITHUB_TOKEN`
uses read access for build evidence and write access for publishing refs and PRs.
The trusted PR verifier has permission to write commit statuses. GitHub settings
must permit:

- **Allow GitHub Actions to create and approve pull requests** under Actions
  settings. The workflow creates PRs; it does not approve its own reviews.
- **Allow auto-merge** and merge commits in repository settings.
- Retained release branches: disable **Automatically delete head branches**.
- On the existing classic `main` branch protection rule, require status checks,
  **Release PR verification**, and **Require branches to be up to date before
  merging**. Keep the PR requirement and any required reviews. The verifier emits
  the status on a PR so it can be selected in the protection settings. If it is
  not listed yet, open a temporary PR to `main` to emit the status, select it as
  required, and close the temporary PR. Non-release PRs fail this check.
- Bot creation/update of `release/*` and creation of release tags. Main does not
  need a bot bypass. The workflow does not modify protection settings.

Final publication refuses to push if auto-merge, branch retention, or the required
classic protection settings are missing. Ruleset-only configurations are not
currently recognized by this check. Repository policy remains authoritative.

Token-created tag pushes do not trigger another push workflow, so draft creation
is explicit. The separate `workflow_run` check verifies the final PR once the
Release run completes, including when token-created PR events are suppressed.

### Failures and recovery

Failed verification, build, signature, or artifact retention changes no remote
refs. A branch/tag conflict rejects the whole atomic push; start a fresh run on
the correct branch. Do not rebase the already-built changes or move a tag.

For a failed draft upload, PR creation, or auto-merge request, choose **Re-run
failed jobs** on the original run, or rerun only **Push and create draft**. It uses
the successful build job's retained package, recognizes the existing identical
refs, and finishes the same release without another bump or build. It refuses to
alter published releases or open a PR for a newer unverified branch head.
After an interrupted final PR step, recover that run rather than launching
Release on the now-final branch, which is deliberately rejected.

Do not use **Re-run all jobs** for draft recovery. Packages and browser diagnostics
expire after 14 days; save the package first if recovery must be delayed.
`node scripts/publish-release.mjs` can recover a saved package from the original
source checkout with `RELEASE_PACKAGE` set to its absolute directory and
`GITHUB_REPOSITORY` set to `owner/repository`, using authenticated Git and GitHub
CLI access. The uploaded `release.json` persists with the GitHub release for
promotion checks, but its referenced workflow run must remain available. A successful retry of
failed publication jobs qualifies the original build record; the latest run
attempt must have completed successfully.
Without the package, exact-artifact recovery is unavailable.

Manual GitHub acceptance covers Mobile branch selection, branch protection,
first/subsequent RCs, final evidence validation, PR auto-merge, retained branches,
and failure recovery. Local tests use isolated repositories and simulated GitHub
responses; they do not establish those live results.

## Supplemental: local preparation and manual release builds

The local commands remain available when preparing a release manually. They do
not replace the primary GitHub Actions procedure above.

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
when someone pushes a final `vX.Y.Z` or RC `vX.Y.Z-rc.N` tag. The name distinguishes
manual preparation/tagging from the RC and Release dispatch workflows; it is
still a tag-push trigger, not a Run workflow control.

It shares verification, build, signature checking, artifact retention, and draft
creation with the primary workflows, but does not prepare a version or change
refs. The tag must match the checked-out application version; development tags
are rejected. Failed draft uploads use the same retained-package recovery.

## Distribution boundary

The automated build applies a local ad-hoc signature and verifies the completed application bundle. That signature supplies bundle integrity but no developer identity: GitHub holds no signing key, and Gatekeeper still requires the user to approve the application manually. Public distribution without that override requires a paid Apple Developer ID certificate and Apple notarization; those credentials are not currently part of the workflow.
