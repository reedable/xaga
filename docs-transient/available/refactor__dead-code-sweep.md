# Library-wide dead-code sweep

## Summary

Public Rust items and exported TypeScript symbols evade the normal unused-code checks. Current `knip` output contains unused exports and files that require consumer verification or tool configuration.

## Requirements

- Verify production, CLI, build, script, fixture, and test consumers before removing or narrowing a symbol.
- Remove dead code; narrow visibility for code with only internal consumers.
- Configure only demonstrated static-analysis false positives.
- Leave the project build, tests, and static checks passing.

## Remaining work

- Audit every externally visible `xaga-core` item against consumers in `xaga-cli` and `src-tauri`.
- Triage the current `knip` report, including unused guards and constants, exported domain/capability types, scripts, fixtures, and configuration files.
- Remove or narrow confirmed findings and configure confirmed tool false positives.
- Re-run Rust checks, TypeScript checks, tests affected by removals, and `knip`.

## References

- `docs/principles/least_visibility_principle.md`
- `crates/xaga-core/src/lib.rs`
- `src/domain.ts`
- `src/types.ts`
- `src/utils/Decorator.ts`
