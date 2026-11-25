# Sign automated version bumps and start PR checks automatically

## Summary

Use a GitHub App to create verified version-bump commits and open PRs whose
required checks start automatically. Apply this to the manual major, minor, and
patch workflows and the first release candidate's automatic patch bump, which
share the version-bump script. The scope is the development-version bump PR.

## Requirements

- Each new automated version bump produces one commit authored by the App's bot
  account and marked **Verified** by GitHub. It contains the synchronized
  version changes and `Cargo.lock` produced by the existing bump command.
- Creating the PR starts **Pull request verification** without anyone approving
  the run or reopening the PR. The required **Verification** result is reported
  on the PR.
- Preserve the existing verification behavior. The workflow checks the PR
  author's identity and compares the proposed merge tree with the generated
  bump. An exact bump from the configured App skips the full suite and keeps
  the **Skipped** conclusion for **Verification**. A modified bump runs the
  full suite.
- Preserve the existing stale-source checks and retry behavior. A retry can
  resume branch or PR creation after a partial failure without duplicating the
  bump or replacing unrelated branch work. Existing PRs remain reusable.
  The signing requirement applies to newly created commits. A retry may reuse
  an exact existing bump commit even if the previous workflow left it unsigned.
  If the PR is missing, open it with the App token so its checks start automatically.
- Use a short-lived installation token scoped to this repository with only the
  permissions needed to create the bump branch, commit, and PR. The App must
  operate under the existing branch protections.

## Remaining work

- Document how to register and install the App and configure its credentials
  and bot identity for the manual and first-RC workflows.
- Use GitHub's `createCommitOnBranch` mutation to publish the generated changes
  as a signed commit. Supply the expected branch head and handle retries after
  branch creation, commit creation, or PR creation. Open the PR using the
  installation token.
- Update PR verification to recognize the configured App and update the release
  guide and workflow summaries for the new behavior.
- Cover the branch and PR recovery paths with automated tests. With the App
  installed in `reedable/xaga`, verify the next real manual bump and first-RC
  bump. For each PR, record the commit's **Verified** badge and the check run
  showing that verification started without approval. Use a separate modified
  bump PR to verify that the full suite runs, then close it without merging.

## References

- [Development, Build, and Release Cycle](../../docs/development/release_cycle.md#start-development-with-github-actions)
- [Shared version-bump script](../../scripts/prepare-version-bump.mjs)
- [PR verification workflow](../../.github/workflows/development-pr.yml)
- [GitHub commit creation and signing](https://docs.github.com/en/graphql/reference/commits#createcommitonbranch)
- [Triggering workflows from a workflow](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow#triggering-a-workflow-from-a-workflow)
