# Prepare release candidates in GitHub Actions

## Summary

Prepare an RC through GitHub's Run workflow control. Commit and push the version
change and tag only after verification and the RC build succeed, so a failed
build does not consume a candidate number.

## Requirements

- Support GitHub Mobile's Run workflow control via `workflow_dispatch`. Run on a
  selected repository branch using `GITHUB_TOKEN` and a shared bot identity; no
  additional credential is required.
- Apply the existing RC version rules in the temporary checkout. Run the existing
  release checks, build the distributable, and verify its signature before
  creating the version commit and annotated tag. The tagged source tree must
  match the verified build's source tree.
- Push the commit and tag atomically, only if the remote branch still equals the
  starting commit and the tag does not exist. Enforce this at push time, including
  concurrent runs. Otherwise leave both refs unchanged and require a fresh run;
  do not rebase or overwrite changes.
- Create a draft prerelease from the already-built artifacts after the push.
  Invoke this explicitly: a `GITHUB_TOKEN` tag push will not trigger the existing
  release workflow. Publication remains manual.
- If draft creation or upload fails after the push, report that the commit and
  tag exist and allow completing the same draft without another version bump.
- Reuse version, verification, and build logic. Preserve manual tagging and
  final-release behavior.

## Remaining work

- Implement the workflow and test success, failures before and after the push,
  and competing branch or tag updates. Verify GitHub-specific behavior in a
  controlled repository, including starting an RC from GitHub Mobile; identify
  anything that remains unverified.
- Make GitHub Actions the primary RC procedure in the durable release-cycle
  documentation, covering web/mobile invocation, branch selection, failure and
  recovery, and draft inspection/publication. Move local version commands, beta
  builds, and manual tagging to supplemental guidance; remove the local beta
  prerequisite for automated RCs while retaining the native verification boundary.

## References

- [Release lifecycle](../../docs/development/release_cycle.md)
- [Release workflow](../../.github/workflows/release.yml)
- [Version preparation](../../scripts/bump-version.mjs)
- [Tag validation](../../scripts/release-tag.mjs)
