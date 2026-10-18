# Move entryWorker back into a Web Worker

## Summary

`processEntry` parses, sanitizes, runs Readability, and converts article HTML to Markdown on the main UI thread. Its worker bundle accesses the unavailable `document` global; the responsible dependency and supported worker boundary are unresolved.

## Requirements

- Keep untrusted-document parsing and sanitization behavior equivalent.
- Move CPU-heavy processing off the UI thread without moving remote acquisition into the worker.
- Transfer only serializable inputs, outputs, progress, and errors across the worker boundary.
- Preserve cancellation and diagnostic step reporting.
- Do not add browser globals to the worker as misleading stubs.

## Remaining work

- Reproduce the worker bundle failure and identify the dependency and code path that accesses `document`.
- Compare replacing that dependency, isolating the DOM-dependent stage, and using a worker-compatible DOM implementation.
- Select the worker boundary and message contract.
- Implement the worker and verify output parity, cancellation, failures, and main-thread responsiveness on large articles.

## References

- `src/text/entryWorker.ts`
- `src/hooks/useArticles.tsx`
- `src/text/entryWorker.test.ts`
- `docs/architecture/security/handling_remote_content.md`
