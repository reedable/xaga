# Sites

A Site groups Entries by normalized article hostname. It has no independently
managed lifecycle or saved membership record. Feed and Topic have their own
records as ManagedCollections; Site is calculated from existing Entries. A Site
identifies a hostname, not a publisher or an ownership relationship. See [Collections and identity](organization_model.md#collections-and-identity).

## Identity

The core `normalize_site_hostname` function derives Site identity from an Entry's
stored article URL. It accepts only HTTP and HTTPS URLs whose parsed host is a DNS
hostname; malformed URLs, hostname-less URLs, other schemes, and IPv4 or IPv6
literals do not belong to a Site.

The URL parser performs hostname case and international-domain normalization.
After parsing, normalization removes one terminal dot and then treats one leading
`www.` as equivalent to its absence. No other subdomain is collapsed. The resulting
hostname is both the stable identity and displayed title.

The UI domain represents this identity as `Site.id`, with `__kind: "site"` and
summary counts. The platform maps the native `SiteSummary.hostname` field to
`id` and `title`; there is no separate UI `Site.hostname`. Native commands and
stored Site pin references retain their `hostname` fields. Navigation carries
`Site.id` in `siteHostname`, passing it to backend hostname parameters.

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

## Shortcuts

A Site can be pinned through the selected Site’s Settings dialog. Only its
normalized hostname is stored in the shared [`pins.yaml` store](pins.md); Site
membership remains derived. A pinned Site with no currently derived Entries
remains navigable with zero total and unread counts. Pin changes never create a
Site file or an entity cache row, and unpinning does not change Entry data or
the current reading scope. Site pin writes use the shared writer and read-only checks.

Site targets require library version **0.12** and the explicit
[CLI upgrade](../library_upgrade.md#site-pins-012) from older library versions.
