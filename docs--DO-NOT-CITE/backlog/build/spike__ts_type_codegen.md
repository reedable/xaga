# Decide Rust-to-TypeScript type generation

## Summary

Rust defines domain and application event names and payloads while `src/platform/events.ts` mirrors the names manually. Rust and TypeScript each validate their own side, but no build check proves the two boundaries agree.

## Requirements

- Establish one authoritative definition for every generated boundary shape.
- Preserve runtime validation and type-brand minting at the untrusted IPC boundary; generated compile-time types do not replace Zod schemas.
- Fail the build when generated output is stale.
- Keep generated files deterministic and reviewable.
- Avoid generating internal domain types that do not cross IPC.

## Remaining work

- Decide whether generation covers event names only, event payloads, or all Tauri command signatures.
- Compare `ts-rs`, `specta`, and `tauri-specta` for Tauri v2 support, runtime-schema interaction, dependency cost, and maintenance.
- Prototype the smallest scope and identify which handwritten constants and types it replaces.
- Present generation and explicit cross-language verification alternatives for decision.
- Implement the chosen boundary and wire stale-output detection into the build.

## References

- `crates/xaga-core/src/events.rs`
- `src-tauri/src/events.rs`
- `src/platform/events.ts`
- `src/platform/TauriPlatform.ts`
- `docs/architecture/type_identity.md`
