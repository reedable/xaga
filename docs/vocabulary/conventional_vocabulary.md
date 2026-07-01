# Conventional Vocabulary

Conventional vocabulary collects words used across the Xaga codebase that are conventional rather than semantic. Unlike the verbs in [Semantic Vocabulary](semantic_vocabulary.md), these do not carry a single honest meaning independent of context — their meaning is relative to the layer they act in.

The test that separates the two: **a word is semantic when it pins the boundary it crosses regardless of how many layers exist; it is conventional when it depends on an implicit single-layer assumption.** A conventional word names only one side of a layer crossing and leans on there being a single implied other side — which holds in a single-layer context but turns into ambiguity in the multi-layered backend. The semantic survivors (`<source>_to_<target>`, `parse`/`serialize`/`deserialize`/`format`) name both ends or a specific boundary, so they stay unambiguous at any layer count.

These words are documented here so their usage stays consistent, not because they name one operation.

## Words

### `load`

A hybrid, layer-relative word. It has no single meaning; what it denotes depends on the layer that uses it.

- **Persistence layer** — Reads raw content from a source and constructs a typed in-memory object from it (read + deserialize). The materialized counterpart to `save`.
  - `load_settings(path) -> UserSettings`, cli `load() -> Config`, model `load(model_dir) -> Result<Self>`, `load_default_embedder() -> Result<Arc<Embedder>>`
- **Runtime / view** — Brings a resource into the runtime or view in a ready-to-use state.
  - `load_article`, `load_via_webview` (render a page into a webview)

Because every value crossing into the UI layer is already a domain object, "produces a typed object" does not distinguish `load` from `read`/`list`/`find`/`fetch` there; the distinction only exists at the persistence boundary where raw content exists. Error behavior is also not uniform (settings loads fall back to defaults; model loads return `Result`).

### `map`

A UI-tier mapper convention: `map<target>` converts a wire/JSON payload into a domain type (e.g. `mapEntry`). It names only the target — the source is implicit.

That is safe only where there is exactly one possible source. The UI tier qualifies: the source is always the wire payload. The backend does not — it has multiple layers (`bytes → storage representation → domain representation`), so a target-only name like `mapEntry` is ambiguous about which conversion it performs. The form's safety is therefore layer-relative, which is why it is conventional rather than semantic.

In the backend, use the semantic `<source>_to_<target>` form (see [Conversion / Mapping](semantic_vocabulary.md)), which names both ends and survives any number of layers. Note `map<target>` composes with — and does not collide with — the functional `map` (`items.map(mapEntry)`): the latter is always a method, the former always a suffixed free function.
