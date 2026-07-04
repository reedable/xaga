# Library-wide dead-code sweep (pending)

## Why the compilers don't catch it

Rust's `dead_code` lint ignores unused **`pub`** items in a lib crate (`xaga-core` is a lib, so
a `pub fn` with zero callers across `xaga-cli` + `src-tauri` is never flagged). TS
`noUnusedLocals` ignores exported-but-unimported symbols.

## The work

- **Rust:** for each `pub` item in `xaga-core`, check for a caller outside its own def/tests in
  `crates/xaga-cli` + `src-tauri`. Some are legitimately CLI-only (diagnostics
  `embeddings list`/`status`/`sample`, maintenance commands) — CLI-used ≠ dead.
- **TS:** triage `npx knip` output (knip is installed). First pass flagged: unused file
  `src/hooks/useSaved.tsx` (verify + remove); unused export `DEFAULT_WAIT`
  (`src/utils/Decorator.ts`); several unused type guards in `src/types.ts`
  (`isTopicSort`/`isHtmlReaderView`/`isItem`/`isEmbedded`/`isShortcut`/`isDocument`/
  `isSavedAnnotation`/`isAnnotationSummary` — confirm none are test-only before removing);
  over-exported types used only within their module (`WebResource`/`Document`/
  `CollectionItem` — can drop the `export`).
- **Knip false positives to configure out:** `postcss.config.js` + `autoprefixer`/`prettier`
  (config-time, used by the build).

The first-pass findings above are from an earlier session; re-verify before removing. Related:
`docs/principles/least_visibility_principle.md` (narrow visibility restores the lints).
