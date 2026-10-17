# Spike: URL normalization and identity stability

## Summary

Resolve scheme validation and HTTP handling in `normalize_url` without silently
changing persisted feed and document identities.

## Requirements

- Reject unsupported URL schemes before they are persisted as feeds or bookmarks.
- Define how an explicitly entered `http://` URL relates to `http_allowed` and
  the current unconditional HTTP-to-HTTPS rewrite.
- Keep identity derivation deterministic. A mutable setting must not change the
  ID produced for the same input URL.
- Any change that alters normalization of previously accepted URLs must include
  migration of affected files and database rows.

## Remaining work

- Choose where HTTP(S)-only validation belongs: centrally in `normalize_url`, or
  at each boundary that accepts a remote URL. Account for all existing callers,
  not only feed and bookmark imports.
- Choose the intended HTTP behavior:
  - retain unconditional HTTPS normalization, with `http_allowed` governing only
    fetch-time HTTP such as redirects;
  - preserve the entered scheme and migrate identities affected by the change; or
  - separate the stable identity URL from the URL used for retrieval.
- Identify which existing persisted URLs would normalize differently under each
  option. If the selected behavior re-keys valid data, define file renames,
  database re-keying, collision handling, and recovery ordering.
- Present the options, affected data, and migration cost for a user decision.
  Replace this spike with implementation and migration tasks after that decision.

## References

The identity derivation is:

```text
input URL → normalize_url → first 12 hex characters of SHA-256 → URL-derived ID
```

That ID names feed `.atom` and document `.md` files and participates in database
keys and fallback feed-entry IDs. Changing normalized output can make existing
data unreachable under its old filename or key; it is not a code-only change.

Current normalization parses with `url::Url`, removes fragments, rewrites HTTP to
HTTPS, removes a leading `www.` where applicable, and removes a trailing slash.
It accepts other parseable schemes such as `file:`, `ftp:`, and `data:`. Current
fetch boundaries prevent those schemes from becoming a local-file read path, but
imports can still persist unusable URLs.

Restricting accepted schemes to HTTP(S) would exclude those unusable records
without changing the normalized output of valid web URLs.

`http_allowed` is enforced by the network proxy during retrieval. It does not
alter normalization, so enabling it does not preserve an HTTP URL that passes
through `normalize_url`. The unconditional rewrite forces TLS for initial URLs;
preserving HTTP would honor that setting but permit plaintext storage and fetches.
Making the rewrite conditional on the setting would also let the same input URL
produce different IDs as the setting changes.

- [URL normalization and hashing](../../../crates/xaga-core/src/util.rs)
- [Feed import boundary](../../../src-tauri/src/native/feeds.rs)
- [Bookmark import boundary](../../../crates/xaga-core/src/persistence/bookmarks.rs)
- [Remote-content policy](../../../docs/architecture/security/fetching_remote_content.md)
- [Fetch-policy settings task](improvement__user_settings.md)
