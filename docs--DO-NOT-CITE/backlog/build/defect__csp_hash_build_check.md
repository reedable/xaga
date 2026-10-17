# Fail the build on CSP hash drift

## Summary

The bundled CSP allow-lists the reader iframe's inline script by hash. `scripts/csp-hash.mjs` prints the computed hash, but the build does not compare it with `src-tauri/tauri.conf.json`; script changes can therefore produce a bundle whose reader script is blocked.

## Requirements

- Fail the production build when the generated reader-script hash differs from every allowed `script-src` hash.
- Extract the same executable script bytes emitted by `buildEntryHtml`.
- Produce an actionable mismatch message containing the expected configuration value.
- Keep CSP verification independent of network access.

## Remaining work

- Add a check mode that parses the configured CSP and compares its hash with the generated value.
- Wire the check into the production build path before bundling.
- Test matching, mismatching, and missing-hash cases.
- Update the durable CSP documentation with the automated command.

## References

- `scripts/csp-hash.mjs`
- `src/text/buildEntryHtml.ts`
- `src-tauri/tauri.conf.json`
- `docs/architecture/security/handling_persisted_content.md`
