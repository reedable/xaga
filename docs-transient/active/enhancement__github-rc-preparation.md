# Prepare release candidates in GitHub Actions

## Summary

Verify the **Release Candidate** workflow from GitHub Mobile for first and
subsequent candidates.

## Requirements

- Run **Release Candidate** on `main` at `x.y.z-dev` to create
  `release/x.y.z` with `x.y.z-rc.1`. If that release branch already exists,
  select it to run the same workflow at `x.y.z-rc.N` for the next candidate.
- Commit and push the RC version to the release branch before verification and
  building. A successful build creates the RC tag and draft prerelease and
  creates or reuses a pull request into `main`.
- A failed build leaves the RC commit on the release branch. Any existing pull
  request already reflects that commit.

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
