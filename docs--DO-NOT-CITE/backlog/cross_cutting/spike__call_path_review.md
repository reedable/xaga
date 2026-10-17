# Review application and CLI call paths

## Summary

Identify complete UI-to-Tauri-to-core and CLI-to-core routes that have no consumer, duplicate another route, or place work in the wrong layer. Symbol-level dead-code analysis does not detect an unnecessary route whose individual symbols call one another.

## Requirements

- Cover every registered Tauri command, every `Platform` method, and every CLI subcommand.
- Evaluate ownership against `docs/architecture/layer_ownership.md`.
- Distinguish an unused route from a supported route used only by diagnostics, maintenance, or tests.
- Do not remove or move a route until its consumers and ownership are verified.

## Remaining work

- Map each frontend `Platform` method through its Tauri command to core operations and identify unmatched methods or commands.
- Map each CLI subcommand to its core operations and identify duplicated orchestration or presentation logic below the CLI layer.
- Present each unnecessary or misplaced route with its consumers and removal or relocation consequences for decision.
- Remove or relocate the chosen routes and verify affected UI, IPC allowlists, CLI parsing, and tests.

## References

- `docs/architecture/layer_ownership.md`
- `src/platform/Platform.ts`
- `src/platform/TauriPlatform.ts`
- `src-tauri/src/commands/`
- `crates/xaga-cli/src/cmd/`
- `crates/xaga-core/src/`
