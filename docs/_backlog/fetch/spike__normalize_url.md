# `normalize_url` review: scheme handling, the HTTP-allowed tension, and ID stability

`normalize_url` (`crates/xaga-core/src/util.rs`) is small but load-bearing: its
output is hashed into every feed and entry ID, so any change to it is also a
data-migration question. This item collects the open concerns about it.

## Why this function is high-stakes: it feeds the ID

The derivation chain:

```
raw URL → normalize_url → url_hash12 (SHA-256, first 12 hex) → id
```

That `id` is the filename of the source-of-truth files (`{hash12}.atom` for
feeds, `{hash12}.md` for entries) and the primary key of the LanceDB rows
(`feed_entry_id` uses the same path when an entry has no `<id>`). **Any change
to `normalize_url`'s output for a given input changes that input's ID.** The
same feed would then hash to a new filename and a new row key, orphaning the
existing `.atom`/`.md` on disk and the existing LanceDB rows — a data
migration, not a code-only change. This constraint governs every item below.

## 1. Scheme vetting (feed and bookmark imports)

`normalize_url` parses with `url::Url::parse` and rewrites `http`→`https`, but
does **not** reject non-web schemes: `file://`, `ftp://`, `data:` parse
successfully and (not being `http`) pass through unchanged. Both import paths
run every URL through it — feed OPML import (`native::feeds::import_feeds`) and
bookmark import (`persistence/bookmarks.rs`) — so a `file:///etc/passwd` in an
OPML `xmlUrl` or a bookmark `href` would be stored as a feed/entry URL.

Downstream guards blunt the impact (the SSRF fetch client can't fetch
`file://`; the article loader is http/https-only), so it is stored junk rather
than a live read primitive — but it should be rejected at import. Fix
candidate: an `http`/`https` allowlist in `normalize_url` (or at both import
sites), plus a test. Note: adding rejection only *excludes* URLs that are
currently junk anyway, so it does not migrate any legitimate ID.

## 2. The `http`→`https` rewrite vs. the "Allow HTTP" setting

`normalize_url` unconditionally rewrites `http`→`https`. The normalized URL is
not only hashed for the ID — it is also stored and fetched (`import_feeds`
fetches it; entries store it as `source`), so **today an http-only site is
fetched over https and simply fails.** This predates the `http_allowed`
setting (default false; enforced in the SSRF proxy — see
`docs/architecture/security/fetching_remote_content.md`). When `http_allowed` is exposed
in the UI, a user who deliberately enables plaintext may expect an `http://`
feed they add to stay `http://` — but the add-time rewrite silently upgrades
it (and, per the above, an http-only feed would fail regardless). The two
mechanisms currently express opposite intents about the same URL.

Design tension to resolve (not a decided fix):

- **Security implication.** The unconditional rewrite is a security feature: it
  forces TLS regardless of how the URL was entered, and it is *why* finding 2's
  downgrade risk is mostly about redirects, not the initial URL. Making the
  rewrite conditional on `http_allowed` would weaken that: a user who enabled
  HTTP would store genuinely `http://` feeds.
- **ID-stability implication (the hard constraint).** `normalize_url` must be a
  pure function of its input — the ID must not depend on a mutable setting. If
  the rewrite became conditional on `http_allowed`, the *same* feed URL would
  normalize (and therefore hash, and therefore be filed) differently depending
  on the setting's value at add time. Toggling the setting later would not
  move existing IDs, so the store would hold a mix of `http`- and
  `https`-derived IDs for what is logically the same feed — drift, and a
  migration with no clean key. Any change here must keep normalization
  deterministic and setting-independent, or accept a defined one-time
  migration.

These have to be weighed together: the security preference (keep forcing https)
and the ID constraint (normalization must not depend on settings) actually
point the same way — leave the rewrite unconditional — while the UX
expectation (respect my "Allow HTTP" choice) points the other way. Resolving it
may mean the setting governs only fetch-time behavior (redirect/downgrade
policy) and never the stored, hashed URL. Decision open.

## Migration note

If any change to `normalize_url` is taken that alters output for
currently-valid inputs, it needs a defined migration: recompute IDs, rename
`.atom`/`.md` files, and rekey LanceDB rows — reconciled through the recovery
path. Scope and mechanism unspecified here; flagged so no change to this
function is made as if it were code-only.
