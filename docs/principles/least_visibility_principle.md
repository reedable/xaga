# Least Visibility Principle

**Give every symbol the narrowest visibility that compiles. Widen it one step
only when an existing caller needs access. Reconsider visibility whenever a
symbol moves; do not copy its old modifier without checking the new location.**

## Why narrow visibility matters

A modifier that is too narrow causes a local compile error, which identifies the
caller needing access. A modifier that is too broad allows other code to depend
on the symbol. Removing that access later can break those callers.

Public symbols are part of the supported API and limit future refactoring.
Exporting unused symbols can also suppress dead-code warnings because the tools
must allow for callers outside the current module or crate. Keep the exposed API
small and review each increase in visibility. A decrease needs no justification
under this principle.

## Moving a symbol can change its reach

A symbol’s effective visibility is limited by both its own modifier and the
modules or exports through which it is reached. A `pub` item inside a
`pub(crate)` module is not external API. Moving it into a `pub mod` can expose it
without changing the item’s modifier. Check both when moving code.

## Rust layer (`crates/xaga-core`, `crates/xaga-cli`, `src-tauri`)

From narrowest to broadest:

```text
private (no modifier) → pub(super) / pub(in path) → pub(crate) → pub
```

- Use `pub(crate)` for items needed across modules of the same crate. Use `pub`
  only when a consumer crate such as `xaga-cli` or `src-tauri` actually imports
  the item.
- Prefer `pub(crate) mod` for internal modules and deliberate `pub use`
  re-exports for the supported API. [`types.rs`](../../crates/xaga-core/src/types.rs)
  and [`embeddings/mod.rs`](../../crates/xaga-core/src/embeddings/mod.rs) provide
  these public exports. A public submodule would let callers bypass them.
- Keep embedding implementation details such as `embeddings::pipeline` and
  `embeddings::row` at `pub(crate)`; do not re-export them as public API.
- Methods of a public trait are public with the trait. Set visibility on the
  trait; individual methods cannot be narrower.
- Unit tests in the same module can access private items. For cross-module test
  access, use `pub(crate)` when necessary, not `pub`.

## TypeScript layer (`src` and `src/platform`)

TypeScript has no `pub(crate)`: a symbol is local to its file or exported. Keep
symbols local until another file actually imports them, and review imports as
well as exports.

- Do not export a symbol for hypothetical future callers. Check for unused
  exports so an unnecessary export does not hide dead code.
- Do not introduce broad `index.ts` re-export files. The project currently has
  none. If a directory needs a shared public API, export only the required
  symbols deliberately rather than re-exporting everything.
- UI components request native work through [`src/platform`](../../src/platform):
  the `Platform` interface, `TauriPlatform`, and the Tauri command API. They must
  not bypass that interface to access core implementation details. Review a new
  `Platform` method as an API expansion, just as a Rust `pub` change would be.
- Keep class members `#private` or `private` unless callers need them. Use
  `readonly` when the value should not be reassigned.

## Operating checklist

1. Start with private Rust items and unexported TypeScript symbols.
2. Widen one step only for a demonstrated caller.
3. Review every newly public Rust item, TypeScript export, or `Platform` method.
4. Recheck the complete access path when moving code.
5. Prefer narrower access when no caller needs the broader API.
