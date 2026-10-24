# Development, Build, and Release Cycle

## Version lifecycle

The committed application version describes its lifecycle state:

| State             | Version                   | Meaning                                                                      |
| ----------------- | ------------------------- | ---------------------------------------------------------------------------- |
| Development       | `0.8.5-dev`               | Work toward the next release is in progress.                                 |
| Local alpha       | `0.8.5-dev+alpha.9c62ffc` | An invocation-only version identifies a local build of a development commit. |
| Release candidate | `0.8.5-rc.1`              | A distributable candidate for the final `0.8.5` release.                     |
| Final             | `0.8.5`                   | The published release version.                                               |

The normal ordering is:

```text
0.8.4
0.8.5-dev
0.8.5-dev+alpha.<commit>
0.8.5-rc.1
0.8.5-rc.2
0.8.5
0.8.6-dev
```

`-dev` and `-rc.N` are SemVer prerelease identifiers. `+alpha.<commit>` is build metadata: it identifies a local artifact without creating another release stage. Alpha versions are local-only and must not be Git tags or GitHub Releases.

`src-tauri/tauri.conf.json` is the application-version source of truth. `scripts/bump-version.mjs` keeps it synchronized with `package.json` and the `[package]` versions in:

- `src-tauri/Cargo.toml`
- `crates/xaga-core/Cargo.toml`
- `crates/xaga-cli/Cargo.toml`

A subsequent Cargo command refreshes the workspace package versions recorded in `Cargo.lock`.

## Starting development

After publishing a final release, select the kind of next release and enter development immediately:

```sh
npm run bump:patch
npm run bump:minor
npm run bump:major
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

Run the complete script and UI test suite with `npm test`. Run only the Node-based build and release script tests with `npm run test:scripts`, or only the UI tests with `npm run test:ui`.

Create an identifiable production-mode artifact from a clean development commit with:

```sh
npm run build:alpha
```

The alpha build does not edit the committed version. It supplies `x.y.z-dev+alpha.<commit>` to Tauri for that invocation and uses the Alpha application icon. It refuses a dirty working tree, a final version, or a release-candidate version.

The ordinary production build is:

```sh
npm run build
```

It runs the advisory Rust dependency audit and then the Tauri build. The audit report does not stop the build; compilation or bundling failures do.

## Release candidates

From `x.y.z-dev`, create and build the first candidate with:

```sh
npm run release:rc
```

This command requires a clean working tree, changes `x.y.z-dev` to `x.y.z-rc.1` in every version file, and runs a production build. Repeating it from `x.y.z-rc.N` increments the candidate to `x.y.z-rc.(N+1)` and builds again.

Commit the version change before tagging a candidate. Do not tag a candidate that has not been committed and validated. A matching `vX.Y.Z-rc.N` tag starts GitHub release automation and creates a draft marked as a prerelease.

## Finalizing a release

From either `x.y.z-dev` or `x.y.z-rc.N`, finalize and build with:

```sh
npm run release
```

This command requires a clean working tree, removes the prerelease identifier, synchronizes the final `x.y.z` version across the version files, and runs a production build. It is rejected when the current version is already final.

After the build succeeds:

1. Review the version-file and `Cargo.lock` changes.
2. Commit the final version.
3. Create an annotated `vX.Y.Z` tag on that commit.
4. Push the commit and tag to GitHub.

For example:

```sh
git add package.json package-lock.json Cargo.lock \
  src-tauri/tauri.conf.json src-tauri/Cargo.toml \
  crates/xaga-core/Cargo.toml crates/xaga-cli/Cargo.toml
git commit -m "Release v0.8.5"
git tag -a v0.8.5 -m "Xaga v0.8.5"
git push <github-remote> <branch>
git push <github-remote> v0.8.5
```

The tag must point to the commit containing both the final version and the release workflow.

## GitHub release automation

`.github/workflows/release.yml` handles final `vX.Y.Z` tags and release-candidate `vX.Y.Z-rc.N` tags. It rejects development tags, including a `vX.Y.Z-dev` tag that matches the committed application version. An accepted tag starts this sequence:

1. Check out the tagged commit.
2. Require a final or numbered-RC tag whose version equals `src-tauri/tauri.conf.json`.
3. Run JavaScript tests and linting.
4. Check the Rust workspace with the lockfile.
5. Build a universal macOS application for Apple Silicon and Intel.
6. Create a draft GitHub Release and upload the build artifacts.

The draft must be inspected before it is published. RC drafts are marked as GitHub prereleases; final drafts are not. The workflow intentionally has no form inputs: the committed version and pushed tag identify the release.

The build matrix contains only macOS while macOS is the only supported platform. Linux and Windows belong in the same matrix after the application supports and is tested on those platforms.

## Distribution boundary

The automated build does not yet establish Apple code signing or notarization. A draft artifact can be used to validate the pipeline, but public macOS distribution must account for Gatekeeper and the project's signing policy before the draft is published.
