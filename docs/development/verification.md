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
