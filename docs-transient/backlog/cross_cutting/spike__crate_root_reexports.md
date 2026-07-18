# Crate-root re-exports create dual paths (pending — unapproved mechanism awaiting decision)

`pub use persistence::{...}` / `pub use query::{...}` in `crates/xaga-core/src/lib.rs` make modules reachable by two paths (e.g. `crate::entries` and `crate::persistence::entries`). The re-exports were added as a compatibility shim during the module reorganization; this specific mechanism was never approved. Decision: keep the shim or repoint all callers to the canonical paths and remove it. Related: `docs/principles/least_visibility_principle.md` (the shim widens the reachable surface).
