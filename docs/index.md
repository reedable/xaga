# Docs

The map of `docs/`. Each directory has its own index; keep this one terse — detail lives in
the directories, not here.

## Durable

What governs, describes, and names the system. Long-lived; code and durable docs may cite
these.

1. [Principles](principles/index.md) — what governs decisions: decision ownership, least
   visibility, secure by default, single source of truth, carried intent, priority, DRY.

2. [Architecture](architecture/index.md) — what the system is and how it behaves: crash
   consistency, design scale, layer ownership, type identity, the single-writer lock, and
   the security-critical paths in [architecture/security/](architecture/security/).

3. [Conventions](conventions/index.md) — how the code speaks: the semantic and conventional
   vocabularies for naming.

## Transient

Work in motion. Items move between these directories and are deleted when resolved; nothing
durable — no code comment, no doc under the three directories above — may cite them.

- `_sprint/` — items pulled into active work.
- `_backlog/` — recorded items waiting: one fact-stated defect, debt, or improvement per
  file, with cited code locations.
