# Verification workflow

Before editing, follow the root `AGENTS.md`: read the
[engineering principles](../principles/index.md) first, then the architecture,
conventions, and development indexes. Follow their links to the documents
applicable to the work. The absence of a requirement in an individual task does
not waive the shared rules.

Choose checks using `package.json`, the relevant development documentation, and
the shared release pipeline in `.github/workflows/release-pipeline.yml`. A generic Git whitespace
check does not establish compliance with the repository formatter. Follow the
shared [formatting requirements](../conventions/formatting.md) for every edit,
and run behavior checks appropriate to the change.

Base completion on check results obtained after the final relevant edits. If a
required check fails, resolve the failure before closing the story. If a
required check cannot run, follow the [verification handoff](https://github.com/reedable/xaga-stories#verification-handoff).
Passing checks establish only what those checks cover.

## Completion gate

Before the final formatting check for each story, review affected documentation
against the implementation. Search the relevant architecture and development
docs, User Guide, and training for names, controls, or behavior the story
changed. Check each match in context and update descriptions that no longer
match the implementation.

For prose-only documentation changes, including ideas, task documents, and
repository instructions, review the content against the request and applicable
requirements and follow the shared [formatting requirements](../conventions/formatting.md).
That is the completion gate for these changes; do not run compilation,
behavioral tests, or `npm run verify:completion`. Determine scope from the work
being completed, not unrelated changes already present in the working tree.
Changes to executable examples, doctests, generated inputs, or configuration
that affects builds or runtime behavior are not prose-only changes.

For changes that affect behavior, choose verification proportionate to what the
change can affect. Identify the affected behavior and dependencies, then run the
checks that establish those requirements. Include relevant checks for each part
of a mixed change; accompanying documentation does not expand the behavioral
testing scope. Determine scope from the work being completed, not unrelated
changes already present in the working tree.

Broaden verification when affected dependencies, failures, or unresolved
uncertainty provide a concrete reason. Do not run unrelated suites merely because
a change includes executable code or configuration. Use `npm run verify:completion`
when the affected scope warrants the full repository suite or another applicable
requirement explicitly calls for it.

The full command runs lint with warnings rejected, repository formatting,
TypeScript, reader CSP verification, Rust formatting, browser tests, all-target
workspace compilation, headless tests, and native tests. Rust compiler and
documentation warnings are errors throughout the command. It stops at the first
failure and requires the documented macOS desktop test environment.

Run the selected checks after the final relevant edits and before claiming
completion or deleting a resolved task. Verify each task's acceptance criteria,
documentation, applicable native smoke procedures, and the
[least-visibility checklist](../principles/least_visibility_principle.md#operating-checklist).
Compiler success alone cannot establish that a public symbol needs its visibility.
Report what was verified and any unmet requirement or unavailable check.

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

## Native shutdown verification

`cargo test -p xaga --test window_shutdown --locked` runs the production
main-window close and Quit handlers in isolated native child processes. It holds
startup open while a write is pending, repeats close and Quit, verifies the saved
article, and checks that a restarted successor acquires the library lock.
The fixture keeps a command's writer handle alive through close, Quit, and
restart.
Read-only close must leave the fixture directory untouched. The test uses blank
webviews and temporary libraries, without opening account settings or user data.
It is included in `npm run test:gui` and requires a macOS desktop session.

`app_menus_theme` exercises the production menu builder and theme event routing,
checks native theme checkmarks, and applies shared theme helpers to existing and
new fixture windows without reading or writing account settings. It is included
in the native suite. It verifies native coordination; frontend theme rendering
is covered separately by UI and browser checks.
