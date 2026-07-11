# Concurrent library writes: annotation era invalidates the old tolerance (pending)

The tolerance for concurrent `.md` writers predates annotations. When it
was decided, the only writes were frontmatter toggles + one-time content
writes — idempotent-ish, last-wins harmless, drift healed by recovery.
Annotations changed the class: entity files are now the *sole* store of
user-authored content (LanceDB rows are partial by design), and every
annotation operation is a whole-file read-modify-write
(`crates/xaga-core/src/persistence/articles.rs`). Two writers interleaving
on one article means the loser's write permanently destroys the winner's
annotation — recovery reconciles FROM files, so a clobbered file is truth
and the loss is unhealable.

## The three writer populations (each needs its own answer)

1. **xaga vs xaga, cross-process.** Lockable — xaga processes can respect
   a lock. Open decision: does the library lock's scope expand from
   LanceDB-writes to library-writes? If write commands refuse without the
   lock, a second instance becomes truly read-only (mark-read fails there —
   a UX change); queueing instead is a bigger mechanism; status quo keeps
   the data-loss window. Undecided.
2. **xaga vs itself, in-process.** Two async commands can race the same
   file today (nothing serializes writes per file). Fixable with in-process
   per-file write serialization — small, no UX change, no cross-process
   machinery.
3. **xaga vs external editors (Obsidian).** Not lockable, by design —
   external editability is a feature, and advisory locks don't bind
   programs that don't check them. This population stays last-wins +
   recovery regardless; the applicable mitigation is detection-shaped
   (re-read/compare before write — cf. the settings ts-at-read design),
   if wanted at all. Undecided.

## Doc wording to settle with the design

`docs/architecture/single_writer_library_lock.md` (and its index entry)
says "at most one writing process per library" and "non-holders read but
do not write" — library-wide, process-level wording for what the owner
states is a LanceDB-scoped guarantee. Whichever way population 1 is
decided, the doc's wording and the decided scope must be made to agree.
