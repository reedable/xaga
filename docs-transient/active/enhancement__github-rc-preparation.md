# Prepare release candidates in GitHub Actions

## Summary

Verify the **Release Candidate** workflow from GitHub Mobile for first and
subsequent candidates.

## Requirements

- Run **Release Candidate** on `main` to create the first candidate. Use
  the development version on `main` or the next patch number when `main`
  carries a final version. If the target release branch already exists,
  select it at `x.y.z-rc.N` to prepare the next candidate.
- Run **Release Candidate** on `main` at `x.y.z-rc.N` to recreate a missing
  release branch with the next candidate.
- Commit the RC version locally before verification and building. After a
  successful build, push the release branch and tag atomically, upload the draft
  prerelease, and leave the release-to-main PR to the final Release workflow.
- A failed build leaves remote refs unchanged so rerunning the build prepares
  the same RC version from the original source.
- First RC publication opens a separate patch-bump PR after pushing the
  release branch if current `main` and the candidate have the same base version.

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
