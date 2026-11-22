# Prepare release candidates in GitHub Actions

## Summary

Verify the **Release Candidate** workflow from GitHub Mobile for first and
subsequent candidates.

## Requirements

- Run **Release Candidate** on `main` at `x.y.z-dev` to create
  `release/x.y.z` with `x.y.z-rc.1`. If that release branch already exists,
  select it to run the same workflow at `x.y.z-rc.N` for the next candidate.
- Commit the RC version locally before verification and building. After a
  successful build, push the release branch and tag atomically, upload the draft
  prerelease, and create or reuse a pull request into `main`.
- A failed build leaves remote refs unchanged so rerunning the build prepares
  the same RC version from the original source.
- First RC publication opens a separate patch-bump PR against current `main`
  after pushing the release branch. Verify recovery of both PRs on GitHub.

## Remaining work

- Verify GitHub Mobile branch selection, first and subsequent candidates,
  artifact retention, permissions, and failure recovery on GitHub.
- Resolve reported failures and obtain a passing final completion gate.

## References

- [Release lifecycle](../../docs/development/release_cycle.md)
- [Release Candidate workflow](../../.github/workflows/prepare-rc.yml)
- [Shared pipeline](../../.github/workflows/release-pipeline.yml)
- [Preparation](../../scripts/prepare-release.mjs)
- [Publication](../../scripts/publish-release.mjs)
