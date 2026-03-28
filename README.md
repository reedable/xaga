# Xaga

Xaga is a Privacy-Focused Personal Knowledge Management (PKM) tool or a Smart RSS Reader.

- Aggregate Content: Fetch web articles via RSS/Atom and convert them into readable formats using readability-js
- Search & Discover: Use the Embeddings and LanceDB components to perform vector-based searches (likely AI-powered) across their saved documents and notes
- Write & Edit: Use CodeMirror 6 to create and modify personal notes saved directly to their local file system (p. 1).

## Architecture

Xaga is a cross-platform desktop application built using Tauri. Xaga has a React-based frontend (User Interface) and a Rust-based backend (File System and Database). The system is designed to handle three primary types of data:

- Feeds: RSS/Atom web content parsed into XML.
- Articles: Read-only .html and .md (Markdown) files.
- Notes: Read-write .md files that the user can edit directly.

## Type Hierarchy

The domain model is expressed identically on both sides of the Tauri boundary.

### TypeScript (`src/types.ts`)

```
Item                id, title, created_at, updated_at
|
├── Document        + content
|
└── Entry           + source, summary, published_at
    └── Article     + content

Feed                id, url, title, entry_count, updated_at
                    (subscribed RSS/Atom feed — not an Item)
```

`Item` is the minimal contract for list views and sorting across mixed collections
(e.g. Search results, Related Items). `Document` marks types whose content is
persisted to disk and indexed in LanceDB — `Entry` is explicitly excluded
until the user saves it as a `Article`.

### Rust (`crates/xaga-core/src/item.rs`)

Mirrors the TypeScript hierarchy using traits and struct composition:

| Rust                                          | TypeScript | Note                                       |
| --------------------------------------------- | ---------- | ------------------------------------------ |
| `trait Item`                                  | `Item`     | `title()`, `date()`                        |
| `trait Document: Item`                        | `Document` | `content()` — gates the embedding pipeline |
| `struct Entry { article: Article, feed_url }` | `Entry`    | `Item` only — not `Document`               |
| `struct Article { meta: Article, content }`   | `Article`  | `#[serde(flatten)]` keeps JSON output flat |

`Article` implements both `Item` and `Document`. `Entry` implements `Item` only.
`Note` will implement both when the Notes pipeline is built.

## xaga-cli

`xaga-cli` can be thought of as a headless implementation of Xaga app. `xaga-cli` uses the Xaga's Rust back-end components without the WebviewWindow for React UI.

## Development Plan

| Phase | Name                   | Description                                                                                                                                                         |
| ----- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | xaga-cli               | Headless Rust backend scaffold — Tauri project setup, CLI entry point, core module structure without WebviewWindow                                                  |
| 1     | Feeds pipeline         | HTTPS fetch of Atom/RSS feeds, XML parsing, CRUD on `*.atom` files                                                                                                  |
| 2     | ArticleLoader pipeline | Tauri scaffold + React UI WebViewWindow + off-screen WebViewWindow (Isolation Pattern, one-time encryption key) + `readability-js` producing `.md` with frontmatter |
| 3     | Articles pipeline      | Synchronous Rust command for read-only `*.md` files, CRUD                                                                                                           |
| 4     | Embeddings pipeline    | `notify` → `Debouncer (200ms)` → `Reconciler` → `flume` → `candle (Metal/CUDA/CPU)` → `LanceDB`; excludes Recovery                                                  |
| 5     | Recovery pipeline      | Startup lazy reconciliation of LanceDB against the filesystem as source of truth                                                                                    |
| 6     | React UI               | Full UI — feed viewer, `unified` remark pipeline (Web Worker), `CodeMirror 6` notes editor, vector search                                                           |
