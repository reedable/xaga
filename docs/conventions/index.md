# Conventions

Conventions for code and documentation, as distinct from what the system is (architecture) and what governs decisions (principles).

## Writing

[Writing style](writing_style.md) — shared language, audience guidance, terminology, and UI copy.

## Documentation

[Documentation](documentation.md) — durable and transient documents, task lifecycle, structure, filenames, and labels.

## Code formatting

[Formatting](formatting.md) — format changes and pass the repository-wide check
before completing every story, including documentation-only work.

[Rust formatting](rust_formatting.md) — apply `rustfmt` consistently across the
Cargo workspace and keep `cargo fmt --all --check` passing.

## Testing

[Automated testing](automated_testing.md) — assert the behavior under test without
overspecifying it; do not hardcode UI copy in behavior tests.

[UI and UX testing](ui_testing.md) — review browser coverage for every UI or UX
change, update meaningful workflow tests, and verify native behavior separately.

## Motion

[Motion](motion.md) — static base styles, with animation enabled only when
`prefers-reduced-motion: no-preference` matches.

## UI styling

[UI styling](ui_styling.md) — rem lengths in decimal increments of one sixteenth.

## Vocabulary

Naming rules for symbols, in [vocabulary/](vocabulary/):

1. [Semantic Vocabulary](vocabulary/semantic_vocabulary.md) — consistent meanings for memory, CRUD, queue, and conversion names; use terms callers understand at their level of abstraction.

2. [Conventional Vocabulary](vocabulary/conventional_vocabulary.md) — layer-specific uses of `load` and `map`, and when to name conversion sources and targets explicitly.
