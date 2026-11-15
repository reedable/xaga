# Hotspot sweep

## Purpose and scope

Identify modules that are both frequently changed in Git history and large in
the current codebase. Their intersection can reveal accumulated responsibilities
and scope creep worth investigating for refactoring.

Size and change frequency identify candidates; they do not by themselves prove
a design problem or establish a refactoring priority.

## Cadence

Follow the [maintenance review trigger](index.md#review-trigger). Run when selected
by the user during that review or explicitly requested at another point.

## Procedure

1. Define and report the repository scope, history interval, module boundaries,
   and measures of change frequency and size for the run. Explain exclusions
   such as generated or vendored code and account for renames and mechanical
   changes when interpreting history.
2. Identify frequently touched modules from Git history and large modules from
   the current source. Compare the two sets and report their intersection with
   the underlying measurements and reproducible commands.
3. Read the candidate modules and relevant changes. Investigate whether their
   responsibilities have expanded, whether unrelated concerns change together,
   and whether existing architecture explains their size and activity.
4. Present supported refactoring opportunities with code references, the problem
   each would address, alternatives, costs, and risks. Distinguish observed
   responsibility growth from hypotheses requiring further investigation.
5. Let the user decide which findings warrant work and their priority. Create
   bounded transient tasks for selected investigations or refactorings; do not
   treat a hotspot ranking as approval to refactor.

## Completion criteria

Report the measurement scope, intersection, reviewed candidates, findings, and
limits. Record selected follow-up work in transient tasks and complete the
applicable [verification gate](../development/verification.md#completion-gate).
Retain this procedure for subsequent runs without accumulating run history here.

## References

- [Decision ownership](../principles/decision_ownership.md)
- [Layer ownership](../architecture/layer_ownership.md)
- [Documentation lifecycle](../conventions/documentation.md)
