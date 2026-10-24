# Documentation

## Organization

Durable documentation governs, describes, and names the system. It is long-lived and may be cited by code and other durable documents.

Ideas live under `docs-transient/ideas/`. The directory is a place to hold ideas,
not a document format: an idea may be prose, a drawing, a poem, a code snippet, a
runnable experiment, another kind of artifact, or a directory containing several
of these. Ideas have no required structure, metadata, template, status, or
expectation that they become tasks or durable documentation.

Transient tasks live under `docs-transient/`, contain only remaining work, and
occupy one of four directories:

- **`backlog/`** — items needing review, grooming, splitting, or prerequisite decisions before they can be selected.
- **`deferred/`** — reviewed items deliberately deprioritized from upcoming work.
- **`available/`** — bounded items ready to be picked up without further grooming or unresolved prerequisite decisions.
- **`active/`** — the small set of items currently selected for implementation or investigation.

Task documents live directly in their state directory; do not subdivide lifecycle
directories by architecture, subsystem, or work area.

Review moves a backlog item to `available/` when it is ready for selection or to
`deferred/` when it is deliberately deprioritized. A deferred item moves to
`available/` when reconsidered and made eligible for selection; an available or
active item may return to `deferred/` when explicitly deprioritized. Selection
moves an available item to `active/`. Move tasks rather than copying them between
states. There is no `done` state: delete the task when its remaining work is
resolved rather than retaining a completion report or task history. Code and
durable documentation must not cite transient tasks in any state.

Each documentation directory has an index. Keep indexes terse and focused on navigation; detailed guidance belongs in the linked documents.

## Transient task structure

The purpose of a transient task document is to define the limits of work: what must change, what is out of scope where ambiguous, and how to tell when the task is complete. It is a working specification, not a progress report or a pitch.

Include only information needed to implement or verify the task. State each point once; do not repeat requirements as work items or expand references into explanations. Omit motivational prose and commentary about the document itself. Bound investigations by a specific question or coverage and a completion condition; avoid catch-all work such as “audit other error paths.”

Transient task documents use these sections, in order:

- **Summary** — the problem or desired outcome and the scope of the remaining task.
- **Requirements** — the conditions the remaining work must satisfy.
- **Remaining work** — concrete outstanding changes, investigations, and dependencies. Describe current behavior only when needed to explain a gap.
- **References** — links to relevant durable documentation, code, and related tasks.

Keep transient tasks focused on remaining work. Do not include completed-work summaries, implementation history, or lists of resolved items. Remove work as it is completed. Lasting decisions and descriptions of implemented behavior belong in durable documentation.

## Transient task filenames

Every transient task file uses a BEM-like name. The category is the block and the
story slug is the element:

`<category>__<story-slug>.md`

Use kebab case within the slug. For a small set of closely related stories, use a
shared group slug and distinguish each story with a modifier:

`<category>__<group-slug>--<story-slug>.md`

Use modifiers only when the relationship is close enough that the files benefit
from sorting together; standalone stories do not need one. A category names why
the item exists, not the shape of the work that will close it. Two tasks requiring
the same kind of code change can therefore carry different categories.

The label is one of five words:

- **`compliance`** — raised by a security, accessibility, policy, or legal concern. This category takes precedence over the other four; accessibility is a requirement, not an enhancement.
- **`defect`** — raised because current behavior is defective relative to its documented or intended behavior.
- **`enhancement`** — raised because new or better product capability is wanted, whether the capability is entirely new or changes existing behavior.
- **`refactor`** — raised because of an internal structure or quality concern with a known target and no intended behavior change.
- **`spike`** — raised because an open design decision, unverified assumption, or unmeasured behavior must be resolved before the work can be scoped.

`compliance` wins when more than one category applies. Among the remaining categories,
classify by why the item was raised rather than guessing how it will eventually be
resolved. A spike remains a spike when investigation reveals a small fix; re-file
it under the category matching the now-known work. Use one category per file and split an
item that has two independent origins.

`defect` and `enhancement` avoid `bugfix` and `feature`, which double as
git-flow branch prefixes and Conventional Commits types. `enhancement` uses the
familiar issue-taxonomy term and is narrower than the generic `improvement`, which
can also describe refactoring or other internal quality work.
