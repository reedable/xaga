# Log skipped persisted-record parse failures

## Summary

Library discovery and recovery silently skip records that fail parsing. Add
per-record diagnostic logging so these omissions can be investigated without
waiting for user-facing error-reporting work.

## Requirements

- Log each persisted-record parse failure currently discarded by Markdown and
  Atom discovery and recovery. Include the operation, file path, and useful
  parser diagnostics. Do not dump document content or log raw file contents;
  redact parser excerpts that expose content while retaining error category
  and location where available.
- Use the existing logging infrastructure and warning severity for skipped
  records. Share reporting where paths share parsing/traversal logic, avoiding
  duplicate warnings for the same failure within one traversal. A later run may
  report the same unresolved failure again.
- Preserve existing parsing, skip/continue behavior, traversal reliability,
  recovery deletion rules, read-only behavior, and persisted data. This story
  adds observability rather than changing failure policy.
- Keep user-facing notifications and persistent error presentation in the
  separate error-reporting story. This work does not depend on that story or
  persisted-identity remediation.

## Remaining work

- Inspect shared Markdown/Atom walkers and recovery's direct parsing paths;
  identify parse errors discarded without diagnostics and add shared reporting
  with operation and path context.
- Add focused coverage establishing that malformed Markdown and Atom records
  produce useful warnings without content disclosure, valid records continue
  to be processed, and shared paths do not duplicate a failure warning.
- Document the logging behavior and its limits in durable persisted-content
  documentation, then complete the shared verification gate.

## References

- [Markdown and Atom traversal](../../crates/xaga-core/src/persistence/walk.rs)
- [Recovery parsing](../../crates/xaga-core/src/recovery/embeddings.rs)
- [Feed discovery](../../crates/xaga-core/src/persistence/feeds.rs)
- [Persisted content](../../docs/architecture/security/handling_persisted_content.md)
- [Error reporting](../backlog/defect__silent-drop-error-reporting.md)
- [Identity remediation](defect__persisted-entity-identity.md)
