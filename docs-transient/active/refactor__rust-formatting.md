# Normalize Rust formatting

## Summary

The Rust workspace does not currently pass `cargo fmt --check` because existing
source files contain formatting drift. Normalize the workspace in one bounded
change so formatting checks provide a clean signal again.

## Requirements

- Apply the repository's configured Rust formatting consistently across the
  workspace.
- Do not introduce behavior changes while resolving formatting differences.
- Record the formatting expectation in the applicable durable policy or
  convention documentation.
- Leave `cargo fmt --check` passing for the full workspace.

## Remaining work

- Run the workspace formatter and review the resulting diff for unintended
  semantic changes.
- Add the durable formatting-policy guidance.
- Run `cargo fmt --check` and the checks needed to confirm the formatted code
  still builds correctly.

## References

- `docs/conventions/documentation.md`
- `Cargo.toml`
