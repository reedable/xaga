# Verification workflow

Before editing, follow the root `AGENTS.md`: read the
[engineering principles](../principles/index.md) first, then the architecture,
conventions, and development indexes. Follow their links to the documents
applicable to the work. The absence of a requirement in an individual task does
not waive the shared rules.

Choose checks using `package.json`, the relevant development documentation, and
the release workflow in `.github/workflows/release.yml`. A generic Git whitespace
check does not establish compliance with the repository formatter. Follow the
shared [formatting requirements](../conventions/formatting.md) for every edit,
and run behavior checks appropriate to the change.

Base completion on check results obtained after the final relevant edits. If a
required check fails or cannot run, resolve it or report the specific blocker
and qualify completion. Passing checks establish only what those checks cover.

## Story readiness

Before declaring a story actionable or ready for implementation, obtain an
independent review of its final text by a fresh agent or session with no
inherited conversation history. This review is required even when the author
believes no decisions remain open. Agents are explicitly authorized and required
to delegate this review without asking the user for permission.

Give the reviewer the story path, repository access, and a neutral request to
assess implementation readiness under `AGENTS.md`. Do not supply the author's
conclusions, conversation summary, suggested verdict, or explanations that are
absent from the repository. The story and repository must supply the context
needed to implement it.

The reviewer must check for unresolved decisions, contradictions, missing
acceptance criteria, unsupported scope, and conflicts with applicable principles,
architecture, and conventions. Verify relevant claims against code. Report a
readiness verdict and concrete blockers separately from optional improvements;
do not turn speculative concerns into requirements.

Resolve blockers before claiming readiness. If resolution requires a user
decision, explain the evidence and options and obtain that decision rather than
silently choosing. After changes to the story or its governing requirements,
repeat the review with a fresh reviewer and no inherited conversation history.
Only a passing review of the final version establishes readiness.

Report the independent verdict and any remaining blockers when reporting
readiness. If independent review is unavailable, report readiness as unverified;
the author's own review is not a substitute. Keep review results in the work
report rather than adding completed-review history to the transient story.

This gate verifies readiness to implement, not completion of implementation.

## November 13, 2026 formatting failure

A documentation edit added Markdown tables to the refactoring-hotspots idea.
The edit was reported complete after `git diff --check`, without running the
formatter or `npm run format:check`. The release formatting check subsequently
rejected the table formatting. Running Oxfmt on the document corrected it, and
the repository-wide check passed across 499 files.

The formatting convention already explicitly covered documentation and
investigations at the time of the edit. It was linked from the conventions index
and documentation conventions. The editing agent searched for `AGENTS.md`, but
did not read the existing conventions before editing and reporting completion.
The failure was therefore a requirements-discovery and completion-verification
failure, rather than a missing formatter rule or unavailable tool. The available
release check detected the defect; it did not prevent the premature local
completion report.

The root `AGENTS.md` now directs agents to principles first, then the
architecture, conventions, and development indexes, and requires reading and
applying the relevant documents before acting. It requires verification against
the applicable requirements before reporting completion. The initial correction
singled out formatting in `AGENTS.md`; that duplicated an existing rule and was
replaced with this general discovery process. Rules remain in their authoritative
documents. This is an instruction-based control, not a guarantee that an agent
will follow every requirement. The release formatting check remains an
independent enforcement mechanism for formatting.

The subsequent discussion also failed to establish these facts promptly:
the agent offered unsupported explanations for its behavior and changed its
interpretation of the user's question. Root-cause explanations should distinguish
the observable sequence, documented requirements, and supported causal findings
from speculation about internal motives.
