# Architecture

Standing architecture rules for this project. Each has its own doc; keep this index terse —
detail lives in the linked file, not here.

1. [Crash Consistency](crash_consistency.md) — every multi-step write has a defined crash story:
   ordering decides what survives, checkpoints bound the re-work, and recovery heals the rest
   from the source of truth. Graceful shutdown and hard kill are distinct failure classes with
   distinct coverage.

2. [Design Scale](design_scale.md) — design to the declared design target (~4× expected intake, a
   multi-million-entry corpus), not the observed average. Sizing must hold for the bulk case
   within the target; unsurfaced scale or "good enough" assumptions are debt, recorded as fact
   and location.

3. [Layer Ownership](layer_ownership.md) — xaga-core is pure logic; orchestration lives in the
   command layer; the CLI is presentation, data migration, and database management. A
   capability's home is determined by what it is, not by who first needed it.

4. [Type Identity](type_identity.md) — shape is the source of truth for intrinsic types; a brand
   (`__kind`) is a derived distillation, legitimate only for extrinsic role distinctions.
   Identity is shape AND brand together, never either alone; brands are minted at the wire
   boundary and never trusted over shape.

5. [Single-Writer Library Lock](single_writer_library_lock.md) — at most one writing process per
   library, enforced by an exclusive advisory `flock` on `xaga.lock`; the lock is handle-tied
   (auto-released on any exit, no stale files). Non-holders read but do not write.

## Security

Architecture of the security-critical paths, in [security/](security/). These describe how a
subsystem works and why — decided design and deliberate limits, not findings or status.

1. [Fetching untrusted content](security/fetching_untrusted_content.md) — the network side.
   Three invariants on every outbound request: containment (the SSRF proxy — resolve, validate,
   pin), minimal truthful disclosure (withhold, never misrepresent), bounded resources.

2. [Handling untrusted content](security/handling_untrusted_content.md) — the byte side: feed
   XML and article HTML through parsing, transformation, storage, and rendering.

3. [Command surface](security/command_surface.md) — every registered Tauri command is
   webview-callable with any arguments at any time; the invoke surface is the boundary, and the
   caller must be assumed hostile.

4. [Performance trade-off](security/performance_tradeoff.md) — resource teardown as a privacy
   tool: per-invocation clients and single-use loader webviews are rebuilt deliberately, not
   pooled; conditional-GET validators are rejected as a server-writable tracking channel.
