# Implement release automation

## Summary

Implement the new release workflow.

## Requirements

- Retain the existing Manual release build workflow.
- All development takes place against `main`, which accepts changes only through
  pull requests. Release automation must not push directly to `main`.
- Follow the [RC publication requirements](enhancement__github-rc-preparation.md)
  for first and subsequent candidates and the patch-bump PR.
- **Release** runs only on `release/x.y.z` with an RC version. Remove the RC
  suffix and commit version `x.y.z` locally in the workflow before building that
  commit. Do not push the version commit before the build succeeds.
- After a successful Release build, atomically push the final-version commit to
  the release branch and create the final tag `vX.Y.Z`. Create or reuse the
  release branch's PR to `main`.
- If Release fails before the atomic push, the remote release branch
  remains on the RC. A fresh run starts from that same
  remote RC commit and repeats the local final-version commit and build.

## Remaining work

- Verify the installed GitHub workflows: first and subsequent RCs, failed builds,
  final Release, PR reuse, branch/tag permissions, and failed-publication retry.

## References

- [Release lifecycle](../../docs/development/release_cycle.md)
- [Manual release build](../../.github/workflows/release.yml)
- [Release Candidate](../../.github/workflows/prepare-rc.yml)
- [Release](../../.github/workflows/prepare-release.yml)
- [Shared release pipeline](../../.github/workflows/release-pipeline.yml)
