# Verify final release preparation in GitHub Actions

## Summary

Verify the final **Release** workflow from GitHub Mobile, including access to
the draft and the release-to-main PR.

## Requirements

- Follow the [Release workflow contract](../../docs/development/release_cycle.md#release).
- Verify publication recovery with the original retained package, including
  reuse of an existing open release PR.

## Remaining work

- Verify release branch selection on GitHub Mobile, final tag and draft
  creation, and the release-to-main PR under the repository's branch protection.
- Verify that retrying interrupted publication reuses the retained package and
  that the final tag still identifies the built commit after the PR is merged.

## References

- [Release workflow](../../.github/workflows/prepare-release.yml)
- [Shared pipeline](../../.github/workflows/release-pipeline.yml)
- [Publication recovery](../../docs/development/release_cycle.md#failures-and-recovery)
