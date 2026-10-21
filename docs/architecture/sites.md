# Sites

A Site is a derived grouping of Entries by normalized hostname. It is a browsing
projection, not a persisted entity, publisher identity, or ownership claim.

## Identity

The core `normalize_site_hostname` function derives Site identity from an Entry's
stored article URL. It accepts only HTTP and HTTPS URLs whose parsed host is a DNS
hostname; malformed URLs, hostname-less URLs, other schemes, and IPv4 or IPv6
literals do not belong to a Site.

The URL parser performs hostname case and international-domain normalization.
After parsing, normalization removes one terminal dot and then treats one leading
`www.` as equivalent to its absence. No other subdomain is collapsed. The resulting
hostname is both the stable identity and displayed title.

## Membership and counts

Entry `.md` files own article URLs. LanceDB Entry rows mirror those URLs and state,
and Site queries group those rows at read time. Feed URLs and Feed membership never
participate. Therefore the same Site can contain Entries from unrelated Feeds, and
a one-Entry Site is still represented.

Site membership is not persisted separately. Total and unread counts are computed
from the same Entry-row read as membership, so the summary and filtered Site list
remain rebuildable from Entry files through normal recovery. Live Entry upserts,
state changes, and removals refresh the frontend through the existing embedding and
Entry-state events.

Code: `crates/xaga-core/src/site.rs`, `crates/xaga-core/src/query/sites.rs`,
`src-tauri/src/commands/sites.rs`.
