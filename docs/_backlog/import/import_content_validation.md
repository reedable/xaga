# Content validation for file imports (OPML feeds, HTML bookmarks)

The path-authority fix (finding 5) moved import/export off the webview and into
`src-tauri/src/native/`, so the file path can no longer come from a compromised
renderer. Validation of the file *contents* was deferred and is still open.

Open items:

- **Byte cap on the file read.** `tokio::fs::read_to_string` in
  `native::feeds::import_feeds` / `native::bookmarks::import_bookmarks` is
  unbounded — a multi-GB file loads entirely into memory on one click. A cap
  refuses that while staying well above any legitimate file.
  - Size estimate (order-of-magnitude, not measured): an OPML feed list is
    ~200–250 B per feed; even an extreme 20-year user (~10,000 feeds) is only
    ~2.5 MB, realistic ceiling a few MB. A `bookmarks.html` is ~150–300 B per
    bookmark; an extreme hoarder (~100,000 bookmarks) is ~30 MB, realistic
    ceiling ~50 MB. A single ~64 MB cap clears both comfortably; ~8–16 MB
    (OPML) and ~64 MB (bookmarks) if sized per type. The value is a decision.
Scheme vetting of imported URLs (rejecting `file://` etc.) lives with the
`normalize_url` review — see `docs/_backlog/fetch/normalize_url.md`, which owns that
function because changes to it carry ID/migration consequences.

Decided out of scope:

- **URL-count cap** — not needed. With path authority in Rust the file is
  user-chosen, so a compromised webview cannot aim import at an attacker's
  huge OPML; the fetch-fan-out-amplifier threat is closed. A legit user's
  large list is a performance concern (the Interactive-tier burst, tracked in
  `docs/_sprint/interactive_batch_command.md`), not security, and the byte cap
  transitively bounds URL count anyway.
- **Error hygiene** — moved to `docs/_backlog/cross_cutting/enhance_error_handling.md`.
- **Export** — nothing beyond the (already-fixed) path authority: content is
  app-generated and `build_opml` escapes all fields; size is bounded by the
  user's own library.
