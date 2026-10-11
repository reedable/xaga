# Crate-root re-exports create dual paths

`pub use persistence::{...}` / `pub use query::{...}` in
`crates/xaga-core/src/lib.rs` make modules reachable by two paths, such as
`crate::entries` and `crate::persistence::entries`.

Decide whether to retain the compatibility re-exports or update callers to the
canonical paths and remove them. Apply `docs/principles/least_visibility_principle.md`
when reviewing the exposed surface.
