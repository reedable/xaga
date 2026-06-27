# Least Visibility Principle

**Every symbol is declared with the narrowest visibility that compiles. Visibility is widened by
exactly one step, and only when a concrete consumer proves the need. On every move or refactor,
re-derive the correct visibility instead of preserving the old modifier.**

This is least privilege applied to the type system. It is the cheapest, most durable encapsulation
we have, because the compiler enforces it and it does not rot.

## Why (the part worth remembering)

- **The cost asymmetry decides it.** Too narrow costs a *local* compile error, fixed by bumping one
  rung exactly where the need is proven. Too wide costs *distributed* coupling discovered later,
  after code depends on the surface — and narrowing then is a breaking change. Cheap+local to widen
  beats expensive+spread to narrow. Always start minimal.
- **Every public symbol is a contract you maintain forever.** The exported surface is what you
  *cannot* freely refactor behind. Fewer exports = fewer promises = more freedom to change internals.
- **Over-exposure blinds the compiler.** `pub` (Rust) / `export` (TS) items are assumed reachable
  from outside, so dead-code analysis goes quiet on them. Narrow visibility restores `unused`
  warnings — the tool tells you what's actually dead. Wide visibility hides dead code.
- **Making it work once is easy; making it work a million times without fail is hard.** Managing
  visibility is the first, compiler-checked step toward the second.

## The one rule that's easy to miss

A symbol's **effective reach = `min(its own modifier, its module/export chain)`.** Moving a symbol
changes the second term *without touching the symbol*. A `pub` item is harmless inside a
`pub(crate)` module; move it into a `pub mod` and it is suddenly external API. So:

> Do not carry a visibility modifier along when you move a symbol. Re-derive it at the new location.

Treat **any visibility *increase* in a diff as a reviewable event** — it is the moment coupling is
created. A decrease never needs defending; an increase always does.

---

## Rust layer (`crates/xaga-core`, `crates/xaga-cli`, `src-tauri`)

The ladder, least → most:

```
private (no modifier)  →  pub(super) / pub(in path)  →  pub(crate)  →  pub
```

Practical guidance for this codebase:

- **`pub(crate)` is the workhorse.** Most things that need to be reachable beyond their own module
  are intra-crate, not API. Default new items to `pub(crate)` the moment they leave a module, not
  `pub`.
- **`pub(crate) → pub` is the boundary that matters most.** `pub` is the only step a *consumer crate*
  can see. That single promotion is the contract; scrutinize it hardest. The test for `pub` is not
  "could someone want this" — it's "does `xaga-cli` or `src-tauri` actually import it today."
- **Submodules are not API.** `pub mod` re-exposes raw internal paths even when a facade exists.
  Prefer `pub(crate) mod` for internals and expose a curated surface from the parent via `pub use`.
  This crate already has the right shape to lean on — facades at
  [`types.rs`](../../crates/xaga-core/src/types.rs) (re-exports domain types) and
  [`embeddings/mod.rs`](../../crates/xaga-core/src/embeddings/mod.rs) (re-exports the intended
  embedding API). Make the facade the *only* door; today the submodules are also `pub`, so the door
  is advisory.
- **Engine internals stay `pub(crate)`.** Things like the `embeddings::pipeline` sync/embed
  primitives and `embeddings::row` builders are mechanism, used only within the crate. They should
  not appear in any facade.
- **Trait caveat.** A `pub trait`'s methods are public with it; you cannot narrow a method below its
  trait. Control visibility at the trait, not per-method.
- **Tests.** Unit tests in the same module see private items; this is not a reason to widen anything.
  Reach for `pub(crate)` for cross-module test access, never `pub`.

## TypeScript layer (`src` — React/presentation, `src/platform` — the port)

TypeScript has no `pub(crate)`. The unit of visibility is the **module** (a file) and the
**package** boundary; a symbol is either local to its file or `export`ed. So "one step at a time"
collapses to **local → exported**, and the discipline shifts to *where* you allow imports from.

- **Don't `export` until something in another file imports it.** An un-exported symbol is the TS
  equivalent of private — keep it that way until proven otherwise. Exported-but-unused is the
  `export`-hides-dead-code trap; lint for unused exports.
- **No barrel sprawl.** This project currently has **no `index.ts` re-export barrels** — keep it
  that way. Barrels silently widen visibility (everything they touch becomes reachable from one
  path) and defeat dead-code detection. If a curated public surface is ever needed for a folder,
  add a *deliberate, minimal* barrel — not an "export everything" one.
- **Cross-layer access goes through the port, not internals.** The presentation layer reaches the
  core only through [`src/platform`](../../src/platform) (`Platform` interface + `TauriPlatform`)
  and the Tauri command boundary — never by importing core mechanism. The `Platform` interface is
  the TS analog of the Rust facade: the one declared door. Add to the interface deliberately; widen
  it with the same scrutiny as a `pub(crate) → pub` promotion.
- **Within classes, prefer the narrowest member visibility:** `#private` (hard private) or `private`,
  and `readonly` where it holds. Default members to private; expose methods only when called from
  outside.

## Operating checklist (for future you)

1. New symbol → narrowest visibility that compiles (Rust: `private`; TS: not exported).
2. Need it wider? Widen **one** rung, and only because a real caller exists now — not "might."
3. The promotion that crosses a crate/package boundary (`pub(crate)→pub`, file-local→`export`,
   adding to `Platform`) is the contract. Justify it; it's the expensive one to undo.
4. Moving/refactoring a symbol → recompute its visibility from scratch at the destination.
5. In review/diff, every visibility **increase** is a flag to inspect; a decrease never is.
