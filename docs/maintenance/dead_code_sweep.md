# Library-wide dead-code sweep

## Purpose and scope

Public Rust items and exported TypeScript symbols evade the normal unused-code checks. Use `knip` to identify unused exports and files for consumer verification.

## Cadence

Follow the [maintenance review trigger](index.md#review-trigger). Run when selected
by the user during that review or explicitly requested at another point.

## Requirements

- Verify production, CLI, build, script, fixture, and test consumers before removing or narrowing a symbol.
- Remove dead code; narrow visibility for code with only internal consumers.
- Configure only demonstrated static-analysis false positives.
- Leave the project build, tests, and static checks passing.

## Procedure

- Audit every externally visible `xaga-core` item against consumers in `xaga-cli` and `src-tauri`.
- Triage the current `knip` report, including unused guards and constants, exported domain/capability types, scripts, fixtures, and configuration files.
- Remove or narrow confirmed findings and configure confirmed tool false positives.
- Re-run Rust checks, TypeScript checks, tests affected by removals, and `knip`.

## Completion criteria

Every finding in the selected scope has a verified disposition: removed, narrowed,
a demonstrated tool false positive, or recorded follow-up work. Present required
decisions to the user and create transient tasks for separate follow-up work.
Complete the applicable [verification gate](../development/verification.md#completion-gate)
and report scope, results, unresolved findings, and unavailable checks. Retain this
procedure for subsequent runs.

## References

- [Least visibility](../principles/least_visibility_principle.md)
- `crates/xaga-core/src/lib.rs`
- `src/domain.ts`
- `src/types.ts`
- `src/utils/Decorator.ts`
