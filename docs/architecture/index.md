# Architecture

Architecture rules and subsystem documentation.

1. [Crash Consistency](crash_consistency.md) — write ordering, saved checkpoints, and recovery from interrupted work; distinguishes graceful shutdown from forced termination.

2. [Design Scale](design_scale.md) — the multi-million-Entry design target, performance measurements, and limits on extrapolation.

3. [Layer Ownership](layer_ownership.md) — responsibilities of core file operations, Tauri command coordination, and CLI presentation and migrations.

4. [Type Identity](type_identity.md) — field checks, `__kind` role indicators, and validation when native payloads enter the UI domain model.

5. [Single-Writer Library Lock](single_writer_library_lock.md) — process-level write exclusion, read-only mode, per-file serialization, and shutdown requirements.

6. [Single-Writer Account Lock](single_writer_account_lock.md) — coordination of shared settings and model downloads, including lock transfer during file replacement.

7. [Article loading diagnostics](article_loading_diagnostics.md) — per-attempt stage and failure records, local log/history, proxy attribution, privacy boundaries, and observation limits.

8. [Sites](sites.md) — Site identity is a normalized HTTP(S) Entry hostname; membership and counts are derived at read time from Entry rows, never persisted or inferred from Feeds.

9. [ListView behavior](list_view.md) — read-state changes and unpinning preserve the current browsing context.

10. [Organization model](organization_model.md) — target domain definitions for collections, pins, reading state, color bookmarks, and Entry Tags.

11. [Pins](pins.md) — ordered shortcut references, their shared UI state, serialized writes, and unavailable destinations.

12. [Library version](library_version.md) — per-library major/minor metadata, explicit upgrade state, and runtime read-only selection.

13. [Inspector](article_inspector.md) — shared inspection views, responsive layout, Annotation drawers, and in-memory tracking of saved Topic changes.

14. [Tag storage](tags.md) — library definitions, readable assignments, indexed IDs, transition records, and the 0.14 upgrade.

## Security

Architecture of the security-critical paths, in [security/](security/). These describe how a subsystem works and why — implementation, rationale, and known limits.

1. [Fetching untrusted content](security/fetching_remote_content.md) — the network side. Three invariants on every outbound request: containment (the network proxy — resolve, validate, pin), minimal truthful disclosure (withhold, never misrepresent), bounded resources.

2. [Handling remote content](security/handling_remote_content.md) — the byte side of fetched content: feed XML and article HTML through parsing, transformation, and the write into storage.

3. [Handling persisted content](security/handling_persisted_content.md) — storage is a boundary: reading previously-written content back and rendering it, regardless of whether it originally arrived by remote fetch, user import, or IPC.

4. [Handling user-supplied content](security/handling_user_supplied_content.md) — content the user supplies directly rather than fetches: OPML feed lists and bookmarks.html imports, parsed from a Rust-owned local file, never from the webview.

5. [Command surface](security/command_surface.md) — every registered Tauri command is webview-callable with any arguments at any time; the invoke surface is the boundary, and the caller must be assumed hostile.

6. [Performance trade-off](security/performance_tradeoff.md) — resource teardown as a privacy tool: per-invocation clients and single-use loader webviews are rebuilt, not pooled; conditional-GET validators are not sent (a server-writable tracking channel).

7. [Embedding model trust](security/embedding_model_trust.md) — trust moved in time, not eliminated: every download verifies against a pinned manifest; the chain runs cryptographic → procedural → infrastructural → reputational, ending at the publisher and the app's own binary.

[Feed refresh](feed_refresh.md) — partial outcomes, safe reporting, retry, and startup behavior.
