# Report feed-refresh failures to the user

## Summary

`refresh_all_feeds` logs individual feed failures but returns only successful
feeds, so partial or complete failure can appear successful. Report failures
without discarding successful refreshes.

## Requirements

- Return typed per-feed failure information identifying the feed and a safe,
  actionable error message. Keep detailed diagnostics out of primary UI copy.
- Show failures through the existing shared error presentation, consolidating
  multi-feed failures into one notification identifying affected feeds and
  reasons. Direct users to the existing Refresh action to retry.
- Preserve successful feed updates and existing list-stability behavior. Do not
  claim all feeds refreshed successfully when any failed; a successful refresh
  with no new articles is not an error.
- Surface whole-request failures, including read-only refusal, through the same
  error mechanism. Clear loading indicators on success or failure; navigation
  cancellation must not produce a failure notification.
- Apply reporting to manual and enabled startup refresh. Keep existing security
  settings, timeouts, and refresh scheduling unchanged.

## Remaining work

- Troubleshoot the reported refresh symptom using the local `~/Xaga/arXiv`
  library: refresh appears to do nothing and gives no explanation. All four
  feed files were updated during the observed attempt, with no per-feed failure
  warnings, so this observation does not establish a fetch failure. Distinguish
  no new articles from ingestion/index/list visibility issues; record separate
  follow-up work if the cause falls outside error reporting. Use synthetic
  fixtures for automated failure cases rather than depending on this library.
- Carry failure outcomes through the Tauri/platform boundary and feed hook,
  retaining successful results and using the shared error sink.
- Test mixed success/failure, all failures, whole-request refusal, no-new-article
  success, cancellation, and loading cleanup. Add browser coverage for visible
  reporting and retry; run relevant Rust/component checks and
  `npm run test:browser`.
- Verify native failure delivery separately, report verification gaps, and update
  durable documentation with the refresh outcome contract.

## References

- [Feed commands](../../src-tauri/src/commands/feeds.rs)
- [Feed refresh](../../crates/xaga-core/src/persistence/feeds.rs)
- [Feed hook](../../src/hooks/useFeeds.tsx)
- [Platform boundary](../../src/platform/TauriPlatform.ts)
- [Shared errors](../../src/providers/ErrorSinkProvider.tsx)
