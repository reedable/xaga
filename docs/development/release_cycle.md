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

Use **RC** or **Release** in the repository's Actions view. Both support
**Run workflow** on GitHub's website and in GitHub Mobile. Select the branch to
release, including an emergency patch branch when needed, then start the run.
Tag refs are rejected. You need repository write access and permission to run
Actions. The dispatch workflows must be present on the default branch, and the
selected branch must contain the workflow and supporting scripts.

| Workflow | Starting version            | Prepared version |
| -------- | --------------------------- | ---------------- |
| RC       | `x.y.z-dev`                 | `x.y.z-rc.1`     |
| RC       | `x.y.z-rc.N`                | `x.y.z-rc.(N+1)` |
| Release  | `x.y.z-dev` or `x.y.z-rc.N` | `x.y.z`          |

Both refuse an already-final starting version. Starting the next development
cycle remains a separate choice of major, minor, or patch increment.

### Verification, build, and commit order

The workflows share `.github/workflows/release-pipeline.yml`:

1. Check out the selected starting commit and run the existing version command
   on the clean tree. Stage the synchronized version files without committing.
2. Run lint with warnings rejected, repository formatting, TypeScript, reader CSP,
   Rust formatting and all-target workspace compilation, headless tests, and
   browser tests. Headless and browser suites each have a 30-minute timeout.
3. Build the Apple Silicon macOS distributable with the plain RC or final version
   and committed standard icons, then verify its ad-hoc code signature. Release
   builds do not regenerate tracked icon files. Intel, Linux, and
   Windows builds are not currently supported.
4. Check that the source tree still matches the prepared tree. Create the version
   commit and annotated tag only after successful verification and building.
5. Retain the already-built app archive and disk image, their checksums, and the
   exact commit/tag identity in a workflow artifact for 14 days.
6. Push the branch and tag together atomically. The branch must still equal the
   starting commit and the tag must be absent at the update itself. Competing RC
   and final-release runs cannot overwrite each other's work.
7. Create a draft GitHub Release and upload the retained artifacts. RC drafts are
   prereleases; final drafts are not. Inspect the draft before publishing it.

A local beta build is not a prerequisite for this flow. Native WebKit and bundled
macOS smoke checks still require the documented desktop environment; the CI
headless/browser checks do not establish those results. See
[verification](verification.md) and [resource verification](resource_verification.md).

### Permissions and repository rules

GitHub supplies `GITHUB_TOKEN`; no personal token or additional App credentials
are needed. Only the publication job requests `contents: write`. Commits use
`github-actions[bot]`. Branch and tag protection still apply: a denied update
fails rather than bypassing repository rules. Repositories requiring signed
commits or pull requests need a compatible release policy before using direct
bot pushes; these workflows do not change those settings.

Token-created tag pushes do not trigger another push workflow. The shared
pipeline explicitly performs draft creation after pushing, avoiding a second
build. See [GitHub token behavior](https://docs.github.com/en/actions/concepts/security/github_token).

### Failures and recovery

- Verification, build, signature, or artifact-retention failure leaves the remote
  branch and tags unchanged. Start a fresh run after correcting the cause. An
  unchanged starting version produces the same candidate number.
- If the branch moves or the tag conflicts, the atomic update rejects both refs.
  Start a fresh run from the current branch; do not rebase the already-built
  changes or move a tag.
- If the push or draft upload fails, use **Re-run failed jobs** on the original
  run (or rerun only **Push and create draft**). It downloads the successful build
  job's package, recognizes an already-pushed identical tag, and finishes that
  same draft. It does not bump or rebuild. Existing published releases are not
  modified. The run summary records the remote tag and commit once known.
- Do not use **Re-run all jobs** to recover an interrupted draft: that rebuilds
  and prepares another local commit. Retained packages expire after 14 days;
  download the package beforehand if recovery must be delayed. With an intact
  saved package, `node scripts/publish-release.mjs` can be run from the original source
  checkout with `RELEASE_PACKAGE` set to its absolute directory and
  `GITHUB_REPOSITORY` set to `owner/repository`, using authenticated Git and
  GitHub CLI access. Without that package, the exact-artifact recovery path is
  unavailable; do not move the existing tag or claim a rebuild is the original.

Browser reports and failure diagnostics are also retained for 14 days. The
manual GitHub acceptance checks cover Mobile dispatch, token and
repository rules, a successful draft, and recovery after a failed upload.

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
