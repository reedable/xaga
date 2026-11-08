# Reproduce articles missing after feed refresh

## Summary

A refresh of the arXiv library appeared to do nothing despite updating its four
feed files without per-feed failure warnings. Determine whether the observed
pass had no new articles or whether ingestion, indexing, or ListView visibility
hid newly discovered articles.

## Requirements

- Reproduce with before/after feed membership, Entry files, index membership,
  and the selected collection/filter captured together.
- Preserve the user's library; use an isolated copy for refresh experiments.
- Distinguish expected stable ListView membership and no-new-article success
  from missing persistence/index updates before proposing a fix.

## Remaining work

- Reproduce the historic symptom. A later read-only snapshot found every feed
  membership URL in the index, no pending embeddings, and unread articles; that
  snapshot cannot establish what the earlier refresh discovered or displayed.
- Scope a fix only if the reproduction establishes a defect.

## References

- [Refresh contract](../../docs/architecture/feed_refresh.md)
- [ListView stability](../../docs/architecture/list_view.md)
