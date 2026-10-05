# Rust → TypeScript type codegen (ts-rs / specta)

The event names and payload shapes are defined in rust (`DomainEvent`, `crates/xaga-core/src/events.rs`; `AppEvent`, `src-tauri/src/events.rs`) and mirrored by hand on the TS side (`src/platform/events.ts` — const arrays with derived literal-union types). The mirror is compiler-checked _within_ TS (use-site literals against the union) but nothing checks the union against the rust enums: a name changed or mistyped on either side is silent drift, guarded only by the rust name tests and review.

Question: should the TS side be generated from the rust definitions?

- Candidates: `ts-rs` (derive-based, per-type `.ts` emission), `specta`/`tauri-specta` (Tauri-aware; can also type commands and events end-to-end).
- What it would replace: `src/platform/events.ts`, and potentially the hand-written payload types/schemas for event payloads (interaction with the zod schemas + minting at the boundary — see `docs/architecture/type_identity.md` — needs thought: generated types are compile-time only, while the schemas validate at runtime; they complement rather than replace).
- Dependency hygiene: both are build-time dependencies; same scrutiny as `defect__zod_direct_dependency.md`.
- Scope question: events only, or also command signatures (tauri-specta's main draw)?
