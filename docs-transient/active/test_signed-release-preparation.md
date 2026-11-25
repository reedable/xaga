# Verify GitHub signatures on release preparation commits

## Summary

Verify the signed release preparation implementation at `4e77e96817fb246d257d4ef67a3a52b37bc06c95`
from `origin/defect__signed-release-preparation` during the next authorized
release cycle. Use a source revision containing that commit on the default
branch and on each selected release branch.

## Requirements

- The first RC, a subsequent RC, and the final release create new
  `Prepare v…` commits authored by `xaga-version-bump[bot]`. GitHub reports
  each as Verified.
- Each prepared commit has the selected source as its sole parent and the
  synchronized release version tree. Verification, packaging, the release
  branch and the peeled tag identify the same commit.
- Preparation removes its temporary ref. The release branch and tag advance
  only after verification and packaging.

## Remaining work

1. Confirm that the repository's App installation, Actions variables and
   secret, and branch rules match the [workflow setup](../../docs/development/release_cycle.md#workflow-setup)
   and [App setup](../../docs/development/release_cycle.md#version-bump-github-app).
   Use an account authorized to dispatch workflows and read run artifacts.
2. For the next authorized release, run **Release Candidate** on `main`,
   **Release Candidate** on the resulting `release/x.y.z` branch, and
   **Release** on that release branch. Use the normal draft workflow. Record
   each selected source SHA and release ref values before dispatch. Confirm the
   first RC opens or reuses the next-development PR when applicable.
3. For each run, read `release-state.json` in the prepared-source artifact and
   `release.json` in the package artifact. Compare their commit and tree IDs
   with the restored commit in verification and packaging logs. Before the
   publication job starts, check that the release branch and tag still have
   their original values. After publication, verify both refs and the commit.
   Set `repo`, `sha`, `release_branch`, `tag`, and `temporary_ref` from the
   run, using the fully qualified temporary ref from the preparation log.

   ```sh
   gh api "repos/$repo/commits/$sha" --jq '{sha, author: .author.login, verified: .commit.verification.verified, reason: .commit.verification.reason, parents: [.parents[].sha], tree: .commit.tree.sha}'
   git ls-remote origin "refs/heads/$release_branch" "refs/tags/$tag^{}" "$temporary_ref"
   ```

   Expect `author` to be `xaga-version-bump[bot]`, `verified` to be `true`,
   `parents` to contain only the dispatch SHA, and `tree` to match the prepared
   source and package. Confirm the commit page displays **Verified**. Expect the
   release branch and peeled tag to point to `sha` and the temporary ref to be
   absent. Inspect the version files and `Cargo.lock` at the prepared commit.

4. Record the run URLs, RC and final commit IDs, API output, ref observations,
   and any failure or unavailable check. If preparation leaves a temporary ref,
   follow the [cleanup procedure](../../docs/development/release_cycle.md#signed-preparation-commits).

## References

- [Release workflow](../../.github/workflows/release-pipeline.yml)
- [Verification handoff](../../docs/development/verification.md#verification-handoff)
