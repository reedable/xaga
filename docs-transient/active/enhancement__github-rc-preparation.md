# Prepare release candidates in GitHub Actions

## Summary

Prepare RCs from GitHub Mobile using a Gitflow release branch. Commit and push
only after verification and building succeed; keep RCs out of protected `main`.

## Requirements

- **Start release** accepts only `develop`, captures a fixed `x.y.z-dev` snapshot and creates
  `release/x.y.z`. Reject an existing destination and direct the user to **RC build**.
- **RC build** accepts only existing `release/x.y.z` branches and prepares its first or next candidate using the existing
  version rules. Release fixes belong there; never merge `develop` implicitly.
- Verify and build before committing. Atomically create/update the release branch
  and immutable RC tag, rejecting concurrent destination changes. Never update
  `develop` or `main`, or open an RC PR to `main`.
- Retain artifacts for retry and record the verified commit, tree, tag, and GitHub
  workflow attempt for final promotion. Preserve draft-prerelease publication and
  recovery without rebuilding or consuming another number.
- Share build/version logic, use `GITHUB_TOKEN`, and keep GitHub Actions primary in
  durable documentation, with local release instructions supplemental.

## Remaining work

- User verification on GitHub: Mobile branch selection, first/subsequent RCs,
  branch retention, build evidence, permissions, and failure recovery.
- Resolve reported failures and obtain a passing final completion gate.

## References

- [Release lifecycle](../../docs/development/release_cycle.md)
- [Shared pipeline](../../.github/workflows/release-pipeline.yml)
- [Preparation](../../scripts/prepare-release.mjs)
- [Publication](../../scripts/publish-release.mjs)
