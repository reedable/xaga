# Verify release workflows on GitHub

## Summary

Verify live RC and final publication from GitHub Mobile using
`bcab86fdfd731b7caa2bcfa541fbdee2f9066d85` from
`origin/defect__first-rc-version-error`.

## Requirements

- Use an authenticated GitHub Mobile session and a disposable GitHub repository
  with Actions enabled. Install the specified commit as its default `main`
  at `0.17.6`, with no `release/0.17.7` branch or `v0.17.7*` tags.
- Use a repository with access to the workflow's macOS runner. Reproduce the
  release branch and tag permissions and protected `main` rules used in the
  target repository. Allow Actions to create PRs except during the recovery check.
- Record the repository, tested commit, run and attempt links, relevant branch
  and tag SHAs, retained artifact name, draft and PR links, and any failure or
  unavailable check.

## Remaining work

1. In GitHub Mobile, run **Release Candidate** on `main`. Verify that a
   successful build creates `release/0.17.7` and `v0.17.7-rc.1` at the same
   prepared commit, retains the release package, and creates a draft prerelease
   with the app archive and disk image. Verify that `main` remains at the
   starting commit and that publication skips the patch-bump PR.
2. In GitHub Mobile, run **Release Candidate** on `release/0.17.7`. Verify
   that the branch advances to `0.17.7-rc.2`, the new tag and draft appear,
   and the RC 1 tag remains unchanged.
3. In a second disposable repository, install the same commit with
   `0.17.7-dev` on `main` using a version-bump PR. Run **Release Candidate**
   on `main` and verify that publication also opens a PR for `0.17.8-dev`.
   Merge the RC 1 release branch into `main` through a PR, keeping the
   release branch. Dispatch **Release Candidate** on `main` and verify that
   it stops with the inconsistent RC message and the recovery guide link.
   Delete the release branch in this disposable repository and dispatch
   again on `main`. Verify that publication recreates the branch at RC 2
   and leaves the RC 1 tag unchanged.
   To check build-failure recovery, use an unused development version on
   `main` and a test-only step in `release-pipeline.yml` immediately after
   version preparation that runs `node -p "require('./package.json').version"`
   followed by `exit 1`. Rerun the failed build from the original run. Verify
   that both attempts print the same RC version and neither creates its release
   branch or tag. Record the fixture commit and both attempt logs.
4. In the first repository, disable Actions PR creation and run **Release**
   on `release/0.17.7` from GitHub Mobile. Verify that build and packaging
   succeed, the release branch and `v0.17.7` point to the built final commit,
   the final draft exists, and publication reports the PR creation failure.
   Re-enable Actions PR creation and rerun **Push and create draft** on that run.
   Verify that it uses the original retained package, leaves the final tag
   unchanged, and opens the release-to-`main` PR.
5. Verify reuse of the open release PR by replaying publication with the same
   retained package using the documented CLI recovery procedure. Verify the
   same PR number is returned and the draft still has both assets. Merge the PR
   after required checks pass and confirm that the final tag still points to the
   built commit. Leave the draft unpublished.

## References

- [Release lifecycle and repository settings](../../docs/development/release_cycle.md#prepare-releases-with-github-actions)
- [Publication recovery](../../docs/development/release_cycle.md#release-failures-and-recovery)
