# Documentation

## Organization

Durable documentation governs, describes, and names the system. It is long-lived and may be cited by code and other durable documents.

Transient tasks contain only remaining work and are deleted when resolved. Active tasks live at the transient-document root; deferred tasks live in its backlog. Code and durable documentation must not cite transient tasks.

Each documentation directory has an index. Keep indexes terse and focused on navigation; detailed guidance belongs in the linked documents.

## Transient task structure

The purpose of a transient task document is to define the limits of work: what must change, what is out of scope where ambiguous, and how to tell when the task is complete. It is a working specification, not a progress report or a pitch.

Include only information needed to implement or verify the task. State each point once; do not repeat requirements as work items or expand references into explanations. Omit motivational prose and commentary about the document itself. Bound investigations by a specific question or coverage and a completion condition; avoid catch-all work such as “audit other error paths.”

Transient task documents use these sections, in order:

- **Summary** — the problem or desired outcome and the scope of the remaining task.
- **Requirements** — the conditions the remaining work must satisfy.
- **Remaining work** — concrete outstanding changes, investigations, and dependencies. Describe current behavior only when needed to explain a gap.
- **References** — links to relevant durable documentation, code, and related tasks.

Keep transient tasks focused on remaining work. Do not include completed-work summaries, implementation history, or lists of resolved items. Remove work as it is completed and delete the task when resolved. Lasting decisions and descriptions of implemented behavior belong in durable documentation.

For task filenames and labels, see [Backlog Item Labels](vocabulary/backlog_item_labels.md).
