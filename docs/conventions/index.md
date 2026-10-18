# Conventions

Conventions for code and documentation, as distinct from what the system is (architecture) and what governs decisions (principles).

## Documentation

[Documentation](documentation.md) — durable and transient documents, task lifecycle, structure, filenames, and labels.

## Code formatting

[Rust formatting](rust_formatting.md) — apply `rustfmt` consistently across the
Cargo workspace and keep `cargo fmt --all --check` passing.

## Vocabulary

Naming rules for symbols, in [vocabulary/](vocabulary/):

1. [Semantic Vocabulary](vocabulary/semantic_vocabulary.md) — one honest meaning per word, independent of context: the verb set for memory, CRUD, queues, and composition. Names fit the developer's mental model at their level of abstraction; where strict accuracy and the mental model conflict, favor the mental model.

2. [Conventional Vocabulary](vocabulary/conventional_vocabulary.md) — words whose meaning is layer-relative rather than semantic (`load`, `map`, …), documented so usage stays consistent. The test: a word is semantic when it pins the boundary it crosses regardless of layer count; conventional when it leans on an implicit single-layer assumption.
