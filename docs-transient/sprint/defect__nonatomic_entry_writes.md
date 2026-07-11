# Entry files written non-atomically (pending)

Three entry-file writers use plain `tokio::fs::write`: a crash mid-write
tears the file, permanently losing the article and every annotation on it.
Annotation content (`text[]`, `occurrence[]`, `note`) exists nowhere else —
the LanceDB row is deliberately partial and recovery reconciles *from*
files, so a torn file is unrecoverable data loss. No concurrency required;
only bad timing.

Sites (`crates/xaga-core/src/persistence/articles.rs`):

- article upsert — line 80
- `add_annotation` — line 178
- `remove_annotation` — line 242

`replace_annotation` (line 217) already uses `fs::write_atomic_async`, and
its doc comment shows the crash-atomicity requirement was understood when
it was written — the requirement was applied to one of the four whole-file
writers.

## Fix shape

Mechanical, no open decisions: the three sites adopt
`fs::write_atomic_async`, matching `replace_annotation`. The Crash
Consistency architecture rule (every multi-step write has a defined crash
story) is the governing doc.
