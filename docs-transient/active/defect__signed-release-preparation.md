# Summary

Migrate automated RC and final release preparation commits from unsigned
`github-actions[bot]` commits to GitHub-verified commits attributed to
`xaga-version-bump[bot]`, using the existing version-bump GitHub App.

# Requirements

- Cover first RC preparation from `main`, subsequent RCs from the release
  branch, and final release preparation. Newly generated `Prepare v…` commits
  must be attributed to `xaga-version-bump[bot]` and have a valid signature that
  GitHub reports as Verified. Changing the Git author name and email alone does
  not satisfy this requirement.
- Reuse the existing App configuration and short-lived installation tokens.
  Share commit-creation logic with development version bumps where the same
  operation is needed. Keep credentials out of logs, bundles, and artifacts;
  missing credentials or signing failures must fail without an unsigned fallback.
- Preserve release safety: build and verification use the exact signed commit
  ultimately published, with the selected source as its sole parent and the
  expected synchronized version-file tree. Do not replace or amend the commit
  after verification. Release branches and tags must remain unpublished or
  unchanged until successful verification and packaging, with the existing
  atomic publication, conflict rejection, and retry guarantees preserved.
- If GitHub-side commit creation requires a temporary remote ref, isolate it
  from release and version branches, handle concurrent runs and cleanup on
  success or failure, and document that preparation can expose source through
  that temporary ref before release publication.
- Preserve development bump behavior and first-RC next-development PR creation.
  Existing release tags, published commits, and PR #68 history are outside this
  migration; do not rewrite them.
- Update release documentation for App permissions, branch rules, token use,
  signing, preparation side effects, and recovery behavior affected by the
  implementation. Do not conflate Git commit signing with application signing.
- Automated coverage must establish App-authenticated commit creation, signed
  commit identity propagation, all preparation modes, credential/API failures,
  conflicts, and publication retry behavior. Simulated API responses cannot
  establish GitHub signature verification: verify a real RC and final release
  commit's App attribution and GitHub verification result, recording the commit
  IDs and evidence. Follow the verification handoff if live checks are unavailable.

# Remaining work

- Replace the local unsigned commit path in release preparation and wire App
  credentials through the necessary workflow jobs. Use the existing
  GitHub-signed commit API approach where compatible with release safeguards.
- Adapt source bundles, release identity validation, publication, and recovery
  only as needed to carry the signed commit unchanged through the pipeline.
- Extend the release and version-bump script tests, update durable development
  documentation, and complete automated and live verification.

# References

- [Release cycle and App setup](../../docs/development/release_cycle.md)
- [Verification and handoff](../../docs/development/verification.md)
- [DRY](../../docs/principles/dry.md)
- [Release preparation](../../scripts/prepare-release.mjs)
- [Version-bump commit creation](../../scripts/prepare-version-bump.mjs)
- [Release publication](../../scripts/publish-release.mjs)
- [Release pipeline](../../.github/workflows/release-pipeline.yml)
- [Release workflow tests](../../scripts/release-workflow.test.mjs)
- [Version-bump tests](../../scripts/prepare-version-bump.test.mjs)
