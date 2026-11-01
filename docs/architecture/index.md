# Architecture

Standing architecture rules for this project. Each has its own doc; keep this index terse — detail lives in the linked file, not here.

1. [Crash Consistency](crash_consistency.md) — every multi-step write has a defined crash story: ordering decides what survives, checkpoints bound the re-work, and recovery heals the rest from the source of truth. Graceful shutdown and hard kill are distinct failure classes with distinct coverage.

2. [Design Scale](design_scale.md) — design to the declared design target (~4× expected intake, a multi-million-entry corpus), not the observed average. Sizing must hold for the bulk case within the target; unsurfaced scale or "good enough" assumptions are debt, recorded as fact and location.

3. [Layer Ownership](layer_ownership.md) — xaga-core is pure logic; orchestration lives in the command layer; the CLI is presentation, data migration, and database management. A capability's home is determined by what it is, not by who first needed it.

4. [Type Identity](type_identity.md) — shape is the source of truth for intrinsic types; a brand (`__kind`) is a derived distillation, legitimate only for extrinsic role distinctions. Identity is shape AND brand together, never either alone; brands are minted at the wire boundary and never trusted over shape.

5. [Single-Writer Library Lock](single_writer_library_lock.md) — at most one writing process per library, enforced by an exclusive advisory `flock` on `xaga.lock`; the lock is handle-tied (auto-released on any exit, no stale files). Its scope is the whole library (`.md` files and LanceDB): every process runs read-write (holds the lock) or read-only (write commands refused, `E_READ_ONLY`); mode is derived from the held lock, never stored.

6. [Single-Writer Account Lock](single_writer_account_lock.md) — the flock on `settings.json` is the account-wide resource lock: the holder has the right to account-scoped resources (settings writes, the model download); writes rotate the lock's inode with holder continuity; writers fail fast, downloaders skip, readers never lock.

7. [Article loading diagnostics](article_loading_diagnostics.md) — per-attempt stage and failure records, local log/history, proxy attribution, privacy boundaries, and observation limits.

8. [Sites](sites.md) — Site identity is a normalized HTTP(S) Entry hostname; membership and counts are derived at read time from Entry rows, never persisted or inferred from Feeds.

9. [ListView behavior](list_view.md) — read-state changes and unpinning preserve the current browsing context.

10. [Organization model](organization_model.md) — target domain definitions for collections, pins, reading state, color bookmarks, and Entry Tags.

11. [Pins](pins.md) — ordered library pin references, shared projection, serialized writes, and unavailable targets.

12. [Library version](library_version.md) — per-library major/minor metadata, explicit upgrade state, and runtime read-only selection.

13. [Inspector](article_inspector.md) — shared Related and Topic views, responsive Article presentation, Annotation drawer, and session membership projection.

14. [Tag storage](tags.md) — library definitions, readable assignments, indexed IDs, transition records, and the 0.14 upgrade.

## Security

Architecture of the security-critical paths, in [security/](security/). These describe how a subsystem works and why — design and limits, not findings or status.

1. [Fetching untrusted content](security/fetching_remote_content.md) — the network side. Three invariants on every outbound request: containment (the network proxy — resolve, validate, pin), minimal truthful disclosure (withhold, never misrepresent), bounded resources.

2. [Handling remote content](security/handling_remote_content.md) — the byte side of fetched content: feed XML and article HTML through parsing, transformation, and the write into storage.

3. [Handling persisted content](security/handling_persisted_content.md) — storage is a boundary: reading previously-written content back and rendering it, regardless of whether it originally arrived by remote fetch, user import, or IPC.

4. [Handling user-supplied content](security/handling_user_supplied_content.md) — content the user supplies directly rather than fetches: OPML feed lists and bookmarks.html imports, parsed from a Rust-owned local file, never from the webview.

5. [Command surface](security/command_surface.md) — every registered Tauri command is webview-callable with any arguments at any time; the invoke surface is the boundary, and the caller must be assumed hostile.

6. [Performance trade-off](security/performance_tradeoff.md) — resource teardown as a privacy tool: per-invocation clients and single-use loader webviews are rebuilt, not pooled; conditional-GET validators are not sent (a server-writable tracking channel).

7. [Embedding model trust](security/embedding_model_trust.md) — trust moved in time, not eliminated: every download verifies against a pinned manifest; the chain runs cryptographic → procedural → infrastructural → reputational, ending at the publisher and the app's own binary.
