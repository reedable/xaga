# Rename Local Build Channels to Alpha and Beta

## Summary

Rename the current `build:alpha` command to `build:beta`, preserving its
validation rules while changing its build metadata to `+beta.<commit>`.
Rename the current `build:dev` command to `build:alpha` and add invocation-only
`+alpha` build metadata without a commit ID.

## Requirements

- Replace the existing `build:alpha` command with `build:beta`. Produce
  `X.Y.Z-rc.N+beta.<commit>` or `X.Y.Z+beta.<commit>` using the existing short
  Git commit hash rules.
- Preserve all existing validation-build rules for beta: require a clean working
  tree, accept only committed final or positive-numbered RC versions, reject
  development versions, and reject any version with a matching local release
  tag, regardless of the tag's target. Do not fetch, move, or delete tags.
- Rename `build:dev` to `build:alpha`. Retain its ordinary Tauri build behavior
  and allow local work in progress without introducing beta's clean-tree,
  committed-version, or untagged-version restrictions.
- Add exactly `+alpha` to the configured application version for an alpha build,
  without a commit hash or other ID. For example, `0.14.3-dev` builds as
  `0.14.3-dev+alpha`; a final or RC version likewise retains its existing
  prerelease portion and gains `+alpha`.
- Supply both channels' versions to Tauri only for the build invocation. Do not
  modify committed version files or lockfiles, commit, tag, push, or publish.
- Update `npm run build` to keep running the advisory audit followed by the
  renamed ordinary build (`build:alpha`). Preserve beta's existing advisory
  audit and failure behavior.
- Update script names, diagnostics, tests, and development/release documentation
  so local RC/final validation uses `build:beta` and `+beta.<commit>`, while
  ordinary local builds use `build:alpha` and `+alpha`. Remove obsolete
  `build:dev` references.
- Keep release-tag validation, committed version lifecycle, and automated
  release behavior unchanged. Both metadata suffixes identify local artifacts
  and must not become release tags or published release versions.

## Remaining work

- Update the package commands and build helpers, preserving invocation-only
  version overrides.
- Review the existing Alpha icon override during the channel rename and align
  helper naming and documentation with the intended channel presentation.
- Verify beta's accepted/rejected versions, clean-tree and local-tag checks,
  commit metadata, and Tauri configuration override.
- Verify alpha's exact ID-free suffix, development/RC/final inputs, support for
  a dirty working tree, and unchanged source version files.
- Run the affected script tests and update
  `docs/development/release_cycle.md`, including examples and release-validation
  instructions.

## References

- [Development, Build, and Release Cycle](../../docs/development/release_cycle.md)
- `package.json`
- `scripts/build-alpha.mjs`
- `scripts/build-alpha.test.mjs`
- `scripts/alpha-icons.mjs`
- `scripts/alpha-icons.test.mjs`
