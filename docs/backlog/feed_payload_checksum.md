# Skip feed reconciliation when the fetched payload is unchanged

Track a checksum of each feed's fetched `.atom` payload. On refresh, if the
checksum of the newly fetched body matches the last known checksum for that
feed, skip the reconciliation of feed entries entirely — no parse, no
`merge_entries`, no `.atom` rewrite, no `.md` scan, no Interactive-tier jobs.

Scope of the win: this reduces the **local** parsing/reconciliation work (and
the refresh burst it feeds into the Interactive tier — see
`docs/sprint/interactive_batch_command.md`). It deliberately does **not**
optimize away the network cost: the full body still downloads on every
refresh. That network cost is a fair price for privacy — the alternative
(conditional GET with `ETag`/`If-Modified-Since`) hands feed hosts a
server-writable, read-back tracking channel and was rejected for that reason.
See `docs/security/performance_tradeoff.md`.

## Ordering constraint (crash consistency)

The checksum must be saved **after** the atomic write of the `.atom` file to
disk, never before. If the checksum were persisted first and a crash landed
between it and the `.atom` write, the next refresh would fetch the same
payload, see a matching checksum, and skip reconciliation — permanently
masking the entries that were never written. Saved after the `.atom` write,
a crash between the two costs exactly one redundant reconciliation pass on
the next refresh. This mirrors the `.md`-before-`.atom` ordering owned by
`write_atom` (`crates/xaga-core/src/persistence/feeds.rs`).

## Open decisions

- Where the checksum lives (candidate: an app-local `<category>` under
  `TAG_CATEGORY_SCHEME` in the `.atom`, like `pinned` — but note that makes
  "save after the `.atom` write" mean a second write, or a checksum of the
  pre-checksum body; the mechanics need design).
- Checksum input: the raw fetched bytes, or the body after
  `fix_bare_ampersands`. Raw bytes detect any change; normalized bytes avoid
  re-reconciling when only irrelevant bytes change. (Whether that case exists
  in practice is unmeasured.)
