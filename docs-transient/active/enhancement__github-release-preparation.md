# Promote verified release candidates in GitHub Actions

## Summary

Promote a verified RC on its retained release branch, then request a protected
PR auto-merge into `main`. Main receives only verified final releases.

## Requirements

- Support GitHub Mobile branch selection. Accept only `release/x.y.z` whose HEAD
  matches an RC tag and evidence from a successful RC workflow in this repository.
  Reject arbitrary branches, development versions, and source changes after RC.
- Prepare the final version and verify/build before committing and atomically
  pushing the release branch and immutable final tag. Reject concurrent changes.
- Create the final draft from retained artifacts, create/reuse a PR to `main`, and
  request auto-merge for the exact built commit. Respect required checks/reviews;
  require the proposed merge tree to match the successful final build. Never
  execute candidate code in the privileged PR verification workflow.
- Keep the release branch after merging. Final tags stay on that branch's verified
  commits. Final draft publication remains manual after merge and inspection.
- Preserve retry of interrupted publication/PR creation without another version
  bump. Share preparation/build logic and retain Manual release build as the
  supplemental tag-triggered path.
- Update durable documentation with this primary flow, evidence requirements,
  GitHub settings, retained branches, and recovery limitations.

## Remaining work

- User verification on GitHub: Mobile promotion, evidence checks, protection
  settings, PR auto-merge, branch retention, and recovery after publication errors.
- Resolve reported failures and obtain a passing final completion gate.

## References

- [RC preparation](enhancement__github-rc-preparation.md)
- [Release lifecycle](../../docs/development/release_cycle.md)
- [Shared pipeline](../../.github/workflows/release-pipeline.yml)
- [PR verification](../../scripts/verify-release-pr.mjs)
- [Build evidence](../../scripts/release-evidence.mjs)
