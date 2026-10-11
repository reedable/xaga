# Article used as the uniform entry carrier

`entries::write_entry` returns `Article { content: "", annotations: [] }` and
`UpsertEntry` carries it. The domain defines `Article` as a loaded,
content-bearing entry (`crates/xaga-core/src/domain.rs`); a body-less feed entry
is an `Entry`.

Decide whether to use `Article` as the uniform carrier and update the domain
model, or switch body-less paths to `Entry`.
