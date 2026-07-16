# Read-only mode: complete the UI experience (pending)

Raised 2026-07-11 from the runtime-modes work (the read-write/read-only split; see `docs/architecture/single_writer_library_lock.md`). The backend enforcement is complete and is the safety boundary: a read-only instance (one that did not win the library flock) has every library-write command refused centrally with a typed `E_READ_ONLY` error. The UI shipped with a minimal affordance layer; this item is the rest of it.

## What exists today

- `useLibraryReadOnly()` hook (`src/hooks/useLibraryReadOnly.tsx`): initial value from the `is_library_read_only` command + the `library:mode-changed` event.
- Sidebar: "Read-only" badge; Refresh, Add-feed, and Add-topic buttons disabled.
- HtmlReader: text selection does not open the new-annotation tooltip.
- `read_article`'s mark-read silently no-ops backend-side (owner decision: reading stays seamless).

## What still shows write affordances (backend refuses; UI doesn't pre-disable)

Inventory as of writing — verify against current code when picking this up:

- Existing-annotation editing: clicking a highlight opens the tooltip with color/note editing and delete; writes fail with `E_READ_ONLY`.
- Bookmark and mark-read/unread toggles (list item context menus, item view).
- Topic membership actions (add/remove item to topic, TopicMenuButton).
- Feed rename/remove and pin/unpin affordances (sidebar item menus).
- Topic edit/delete dialogs (`EditTopicDialog`), feed dialog (`EditFeedDialog`).
- Import flows (OPML / bookmarks.html) if reachable from menus on a read-only instance.

## The ask

- Decide the presentation per affordance: hide vs disable-with-tooltip (the sidebar buttons chose disable-with-tooltip; MUI disabled buttons need the `Tooltip > span > button` wrapper — the local eslint rule accepts this shape).
- Map `E_READ_ONLY` rejections to a consistent, non-generic user message wherever a write can still be triggered (defense in depth; the error string's prefix is the match key, `src-tauri/src/commands/mod.rs::E_READ_ONLY`).
- A read-only instance shows data as of ITS startup (both `.md` reads and the version-pinned LanceDB read handle): consider surfacing "as of <launch time>" or a manual refresh-view affordance (which would need a reload mechanism — a design question, not just UI).
- Menu bar items that trigger writes (clear history is app-scope and fine; check import/export entries) should reflect the mode.

## Non-goals

- No change to the enforcement model: the IPC gate remains the boundary; the UI layer is presentation only.
- No live cross-process update mechanism (no file watcher; recovery-at-startup is the architecture's answer — owner decision).
