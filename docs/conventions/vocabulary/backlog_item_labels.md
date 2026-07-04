# Backlog Item Labels

Every file under `docs/_backlog/<domain>/` is named `<label>__<slug>.md`. The label is a
closed set of five words. Unlike [Semantic Vocabulary](semantic_vocabulary.md) and
[Conventional Vocabulary](conventional_vocabulary.md), which name what a code symbol _does_,
a backlog label names **why the item exists** — its originator — not the shape of the work
that will eventually close it. Two items can require the same kind of code change and still
carry different labels, because they were raised for different reasons.

## Labels

- **`compliance__`** — raised by a security, policy, or legal concern (all three are
  compliance concerns). Takes precedence over the other four: if a gap is security-,
  policy-, or legal-relevant, it is `compliance__` regardless of whether the eventual fix
  would otherwise read as a defect, an improvement, or a decision.
- **`defect__`** — raised because current behavior is defective relative to its documented or
  intended behavior.
- **`improvement__`** — raised because new capability is wanted, whether that capability is
  entirely new or an enhancement to something that already exists.
- **`refactor__`** — raised because of an internal structure or quality concern (duplication,
  consolidation, restructuring) with a known target and no intended behavior change.
- **`spike__`** — raised because a question needs an answer before the work can even be
  scoped: an open design decision, an unverified assumption, or unmeasured behavior. Catch-all
  for anything not `compliance__`-precedent and not yet decided enough to be a `defect__`,
  `improvement__`, or `refactor__`.

`defect__`/`improvement__` deliberately avoid `bugfix`/`feature`: those words double as
git-flow branch prefixes and Conventional Commits types, and risk being pattern-matched as
"what kind of branch/commit to make" rather than "why this item exists." `improvement__` was
chosen over `enhancement__` for the same reason: several trackers use `enhancement` to mean
only an upgrade to something that already exists, in contrast to a separate "feature request"
label for net-new capability — an inconsistent, contradictory prior for a cold reader.
`improvement` is not institutionalized as a tracker-label category, so it doesn't compete with
this doc's own definition.

## Precedence

`compliance__` is checked first and wins on conflict. An item that is both a security gap and
an open decision (e.g., no port allowlist decided for a proxy) is `compliance__`, not
`spike__` — the originator, not the current decision-status, drives the label. Among the
remaining four, label by why the item was raised, not by guessing how it will eventually be
resolved: an item raised as an open question stays `spike__` even after investigation narrows
it down to a one-line fix, until it is re-filed under the label that matches the now-known
work.

## Format

`<label>__<slug>.md`, double underscore between label and slug, single underscores within the
slug. One label per file — an item that would otherwise need two originators should be split
into separate backlog items.
