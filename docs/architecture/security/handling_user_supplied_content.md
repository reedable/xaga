# Handling user-supplied content: OPML and bookmarks.html import

How Xaga secures content the user supplies directly rather than fetches: an imported OPML feed list or `bookmarks.html` file. Both arrive as a local file the user selects via a Rust-owned file dialog, never as a path or content taken from the webview — see [command_surface.md](command_surface.md) for why that authority segregation exists; this document covers what happens to the file's _content_ once selected, not who is allowed to trigger the read.

Content typed directly by the user through IPC (an annotation, a manually entered feed URL) is not covered here: it has no format-specific parser of its own. A manually entered feed URL becomes Remote content once fetched (see [handling_remote_content.md](handling_remote_content.md)); typed text like annotations goes straight to storage and is covered by [handling_persisted_content.md](handling_persisted_content.md), which treats all stored content identically regardless of provenance.

Code: `src-tauri/src/native/feeds.rs`, `src-tauri/src/native/bookmarks.rs`, `crates/xaga-core/src/persistence/opml.rs`, `crates/xaga-core/src/persistence/bookmarks.rs`.

## Resource bound

`native::feeds::import_feeds` and `native::bookmarks::import_bookmarks` check the file's size via `tokio::fs::metadata` against the `import_max_bytes` setting (default 64 MiB, `DEFAULT_IMPORT_MAX_BYTES`, `src-tauri/src/lib.rs`) and refuse the import before ever reading the file if it's exceeded — the oversized file is never loaded into memory. This is a single cap shared by both formats rather than one per format: a format-specific split would tighten the ceiling for the smaller of the two (OPML), but adds a second number the user must reason about, for a cap whose purpose is protecting the user from an oversized file by accident, not from an adversary shaping one format differently from the other. Settings → General → Maximum import file size edits this cap in MiB. Saving persists the byte count under the account lock and updates the running process; the next feed or bookmark import uses it. The control accepts positive whole MiB and saves valid changes on blur, Enter, or a Settings-window close request. Closing waits for persistence; invalid edits or a refused write keep the window open. The setter requires whole MiB expressed in bytes, within JavaScript’s maximum safe integer.

## OPML (feed list)

`parse_opml` (`persistence/opml.rs`) reads the file with `quick_xml` directly — the same parser family as the Feed XML pipeline, so it inherits the same property: no DTD resolution, no external entity expansion, no path from an imported OPML file to a local-file read or request amplification. Only `xmlUrl` attribute values are extracted; nothing else in the document is interpreted or persisted. Each extracted URL is XML-unescaped, trimmed, and then handed to the same `normalize_url` + `add_feed` path a manually-entered feed URL would use — from that point on, each URL becomes Remote content and falls under `handling_remote_content.md`. A URL-count cap on the parsed output is unnecessary: path authority already prevents a compromised renderer from directing import at an attacker-chosen file, so the fetch-fan-out-amplifier threat a URL-count cap would guard against is already closed; the byte cap above transitively bounds URL count regardless.

## bookmarks.html (Netscape bookmark file)

`bookmarks::parse` (`persistence/bookmarks.rs`) parses the file with `scraper` (html5ever) — a different parser from both the OPML/Atom (quick-xml) and Article HTML (Readability+DOMPurify) pipelines. It selects `dl dt a` elements and reads:

- **`href`** — passed through `normalize_url`, same validation a manually entered feed/article URL gets. Entries whose URL fails to normalize are skipped, not imported.
- **Link text (title) and the following `<dd>` (description)** — read via `.text()`, which yields only text nodes. There is no DOMPurify-equivalent sanitization step here, and none is needed for the same reason the Rendering path doesn't rely on it either: `.text()` discards markup by construction, so no HTML from the bookmarks file can survive into the extracted title/description as anything but inert text.
- **`color`** — accepted only when it matches a shared swatch palette identifier. Missing or unsupported import colors fall back to Blue.
- **`add_date`/`published_at`/`last_modified`** — parsed as integers via `str::parse::<i64>`; unparseable values are silently dropped (`None`), never propagated as an error.

Extracted entries are persisted through `write_read_entry` / `set_entry_color` (`persistence/entries.rs`) in the serialized Entry writer lane, the same strict noyalib field-patch frontmatter boundary that Feed XML and Article HTML entries go through — see `handling_remote_content.md`, "Persistence." Title and description injection into frontmatter is bounded the same way regardless of whether the entry originated from a fetched feed or an imported bookmark.

## What a hostile import file cannot do

- **Choose its own path or bypass the dialog.** Both `import_feeds` and `import_bookmarks` are `pub(crate)` functions, not `#[tauri::command]`s — the file dialog runs in Rust and the chosen path never round-trips through the webview (`native/feeds.rs:1-8`, `native/bookmarks.rs:1-4`). A compromised renderer has no invoke door to redirect an import at an arbitrary file.
- **Aim a fetch at an attacker-controlled address via a huge URL list**, per the OPML section above — the network proxy (`fetching_remote_content.md`) still validates every URL's resolved address regardless of how many the import produces.


## Import outcomes

Native import/export events carry structured errors with a user-safe `message` and an empty `detail`; technical errors are logged before emission. Import results carry a numeric `failed` count rather than failed-entry strings. The frontend reports complete and partial failures through the shared error display and retains successful imports.

Files above the shared byte cap return `import:too-large`, directing users to increase Maximum import file size in Settings → General or use a smaller file.
