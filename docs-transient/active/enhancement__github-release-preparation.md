# Prepare final releases in GitHub Actions

## Summary

Add a final-release counterpart to RC preparation, runnable from GitHub Mobile.
Commit and push the final version and tag only after verification and the build
succeed. Rename the existing tag-triggered workflow to Manual release build.

## Requirements

- Provide a Release workflow using `workflow_dispatch`, supporting GitHub Mobile's
  Run workflow control and branch selection. Use `GITHUB_TOKEN` and the shared bot
  identity without additional credentials.
- Run `npm run version:release` on a clean temporary checkout before building.
  Preserve its existing transitions from `x.y.z-dev` or `x.y.z-rc.N` to `x.y.z`
  and rejection of already-final versions. Keep the prepared changes uncommitted
  through release verification, distributable building, and signature checking.
- After success, commit the prepared version changes and create the annotated
  final tag. The tagged source tree must match the verified build's source tree.
  Push branch and tag atomically only if the branch still equals its starting
  commit and the tag is absent, including competing RC and final-release runs.
  Otherwise leave both remote refs unchanged and require a fresh run.
- Explicitly create a draft release, not a prerelease, from the already-built
  artifacts after the push. Publication remains manual. If draft creation or
  upload fails, allow finishing that same release without preparing another
  version or moving the tag.
- Share preparation, verification, build, push, and recovery logic with the RC
  workflow. Preserve the native verification boundary and manual tagging path.
- Change the existing Release workflow's display name to Manual release build;
  retain its tag-push trigger and support for both RC and final tags.

## Remaining work

- User verification on GitHub: final-release invocation from Mobile, token
  permissions, draft recovery, and the renamed manual tag-build workflow.
  Resolve any reported failures.
- Obtain a passing completion gate after the final changes.

## References

- [RC preparation](enhancement__github-rc-preparation.md)
- [Release lifecycle](../../docs/development/release_cycle.md)
- [Existing release workflow](../../.github/workflows/release.yml)
- [Version preparation](../../scripts/bump-version.mjs)
- [Tag validation](../../scripts/release-tag.mjs)
