# Conventional Vocabulary

These naming conventions depend on the layer using them. Unlike the terms in
[Semantic Vocabulary](semantic_vocabulary.md), a name such as `load` does not
identify one specific operation throughout the codebase. Use these conventions
only where the caller can tell the source and operation from context.

Names such as `<source>_to_<target>`, `parse`, `serialize`, `deserialize`, and
`format` identify a conversion or representation change explicitly. They remain
clear when the code has several intermediate representations.

## Words

### `load`

`load` has two established uses in Xaga:

- **Persistence:** reads raw content and constructs a typed object (read plus
  deserialize). Examples: `load_settings(path) -> UserSettings`, CLI
  `load() -> Config`, model `load(model_dir) -> Result<Self>`, and
  `load_default_embedder() -> Result<Arc<Embedder>>`.
- **Runtime or view:** makes a resource ready to use or display. Examples:
  `load_article` and `load_via_webview`.

The UI receives domain objects already, so “produces a typed object” does not
separate `load` from `read`, `list`, `find`, or `fetch` there. Failure behavior also
depends on the operation: settings loads can fall back to defaults; model loads
return `Result`.

### `map`

In the UI, `map<target>` converts a native JSON payload to a domain object, such
as `mapEntry`. The source is unambiguous there: it is the native payload.

Backend code converts several representations, such as bytes, storage records,
and domain objects. A target-only name would not identify which conversion it
performs. Use `<source>_to_<target>` there, as described under
[Conversion / Mapping](semantic_vocabulary.md#conversion--mapping).

`mapEntry` is a named conversion function; `entities.map(mapEntry)` uses the array’s
`map` method to apply it. The two uses do not conflict.
