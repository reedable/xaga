# Article used as the uniform entry carrier (pending — unapproved change awaiting decision)

`entries::write_entry` returns `Article { content: "", annotations: [] }` and `UpsertEntry` carries it — but the domain defines `Article` as "a loaded, content-bearing Entry" (`crates/xaga-core/src/domain.rs`); a body-less feed entry is an `Entry`. The richer type is used where the base type is accurate. Made during the decoupling work; the domain mismatch was recorded, not approved. Decision: ratify `Article` as the uniform carrier (and update the domain doc) or switch the body-less paths to `Entry`.
