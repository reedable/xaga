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
verify it is available with `protoc --version`. The release workflow installs
this prerequisite explicitly in both its verification and build jobs.

## Release candidates

From `x.y.z-dev`, prepare the first candidate with:

```sh
npm run version:rc
```

This command requires a clean working tree and changes `x.y.z-dev` to `x.y.z-rc.1` in every source version file and in the three corresponding `Cargo.lock` package entries. Repeating it from `x.y.z-rc.N` increments the candidate to `x.y.z-rc.(N+1)`. Review and commit the version files and refreshed lockfile before building or tagging.

After committing an untagged RC or final version, create its local validation build with:

```sh
npm run build:beta
```

The beta build does not edit version files, commit, tag, push, or publish anything. It supplies `x.y.z-rc.N+beta.<commit>` or `x.y.z+beta.<commit>` to Tauri for this invocation and uses the Alpha application icon. It requires a clean working tree and a committed numbered RC or final version so that the artifact corresponds exactly to a commit. Development versions are refused.

Beta builds require that the matching release tag (`vX.Y.Z-rc.N` or `vX.Y.Z`) does not exist locally, regardless of which commit it points to. An untagged final version is allowed; a final version with its release tag is refused, just like a tagged RC. The check uses local tags and does not fetch remote tags. Published tags remain immutable; a beta build does not delete or move them.

Commit the version change before tagging a candidate. Do not tag a candidate that has not been committed and validated. A matching `vX.Y.Z-rc.N` tag starts GitHub release automation and creates a draft marked as a prerelease.

## Finalizing a release

From either `x.y.z-dev` or `x.y.z-rc.N`, prepare the final version with:

```sh
npm run version:release
```

This command requires a clean working tree, removes the prerelease identifier, and synchronizes the final `x.y.z` version across the source version files and `Cargo.lock`. It is rejected when the current version is already final. Review and commit the changes before tagging.

## Tagging a release

Version preparation never commits, tags, pushes, or performs the distributable build. After preparing an RC:

1. Review and commit the synchronized version changes, including the automatically refreshed `Cargo.lock`.
2. Run `npm run build:beta` to validate the committed RC locally.
3. Run `npm run release:tag` from the clean committed tree.
4. Push the commit and tag to GitHub.

For a final version, review and commit the synchronized version files and `Cargo.lock` after `npm run version:release`, then tag the clean committed tree.

For a final release, for example:

```sh
git add package.json Cargo.lock \
  src-tauri/tauri.conf.json src-tauri/Cargo.toml \
  crates/xaga-core/Cargo.toml crates/xaga-cli/Cargo.toml
git commit -m "Release v0.8.5"
npm run release:tag
git push origin <branch> --follow-tags
```

`release:tag` requires a clean tree; verifies the versions in `package.json`, `tauri.conf.json`, the three workspace manifests, and their `Cargo.lock` entries; rejects `-dev`; accepts final and positive-numbered RC versions; and refuses to replace an existing local tag. It creates an annotated `v<version>` tag on `HEAD` but never commits or pushes.

The tag must point to the commit containing both the prepared version and the release workflow. Pushing it delegates tests, the distributable build, and draft GitHub Release creation to GitHub Actions.

## GitHub release automation

`.github/workflows/release.yml` handles final `vX.Y.Z` tags and release-candidate `vX.Y.Z-rc.N` tags. It rejects development tags, including a `vX.Y.Z-dev` tag that matches the committed application version. An accepted tag starts this sequence:

1. Check out the tagged commit.
2. Require a final or numbered-RC tag whose version equals `src-tauri/tauri.conf.json`.
3. Install JavaScript dependencies, then run lint, formatting, frontend type,
   and reader CSP checks.
4. Set up Rust and check workspace formatting before restoring the Rust cache
   and installing the Protocol Buffers compiler required by Lance.
5. Check the Rust workspace with the lockfile and install Chromium.
6. Run `npm run test:ci`: script, UI, Rust, and browser tests for both RC and final
   tags. Preserve browser reports and failure diagnostics for 14 days. Any test
   failure blocks the release build. Native WebKit tests run separately through
   `npm test` on a Mac with a desktop session.
7. Build the macOS application for Apple Silicon (`aarch64`). Intel Macs are not currently supported.
8. Create a draft GitHub Release and upload the build artifacts.
9. Verify that the macOS application bundle has a valid code signature.

The draft must be inspected before it is published. RC drafts are marked as GitHub prereleases; final drafts are not. The workflow intentionally has no form inputs: the committed version and pushed tag identify the release.

The build matrix contains only Apple Silicon macOS while that is the only supported platform and architecture. Linux and Windows belong in the same matrix after the application supports and is tested on those platforms. Intel macOS can be added as a separate artifact if the project later chooses to support it; releases are not universal binaries.

## Distribution boundary

The automated build applies a local ad-hoc signature and verifies the completed application bundle. That signature supplies bundle integrity but no developer identity: GitHub holds no signing key, and Gatekeeper still requires the user to approve the application manually. Public distribution without that override requires a paid Apple Developer ID certificate and Apple notarization; those credentials are not currently part of the workflow.
