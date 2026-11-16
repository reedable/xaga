# Implement release automation

## Summary

Implement the new release workflow.

## Requirements

- Retain the existing Manual release build workflow.
- All development takes place against `main`, which accepts changes only through
  pull requests. Release automation must not push directly to `main`.
- Running **Release Candidate** on `x.y.z-dev` immediately creates
  `release/x.y.z` with a committed version of `x.y.z-rc.1`, then runs the RC
  build against that commit.
- Running **Release Candidate** on `x.y.z-rc.N` checks out the existing
  `release/x.y.z`, commits the next RC version, then runs the RC build against
  that commit.
- Only after a successful RC build, create the RC tag and a PR from
  `release/x.y.z` to `main`. Reuse an existing open PR rather than creating
  another one.
- A failed build leaves the committed RC version on the release branch without
  creating its tag or a new PR. If a PR already exists, pushing the next RC
  commit updates that PR immediately, even if the build later fails.
- **Release** runs only on `release/x.y.z` with an RC version. Remove the RC
  suffix and commit version `x.y.z` locally in the workflow before building that
  commit. Do not push the version commit before the build succeeds.
- After a successful Release build, atomically push the final-version commit to
  the release branch and create the final tag `vX.Y.Z`. The branch push updates
  its existing PR to `main`.
- If Release fails before the atomic push, the remote release branch and PR
  remain on the RC, with no final tag created. A fresh run starts from that same
  remote RC commit and repeats the local final-version commit and build.

## Remaining work

- Implement the release workflow according to the requirements recorded here.

## References

- [Release lifecycle](../../docs/development/release_cycle.md)
- [Manual release build](../../.github/workflows/release.yml)
- [Shared release pipeline](../../.github/workflows/release-pipeline.yml)
