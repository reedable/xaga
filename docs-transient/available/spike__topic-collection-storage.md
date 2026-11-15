# Define Topic storage and identity

## Summary

Topic identity and storage are coupled to `topics/<id>.md`. Topic saves write an `id` field, but Markdown parsing discards it and reads derive the domain ID from the filename. Feed reads likewise use the `.atom` filename rather than the stored Atom ID. Topic and Feed are both modeled as Collections, but it is undecided whether that common domain shape warrants a common storage representation.

## Requirements

- Define Topic identity independently of its storage location and file extension.
- Derive storage location from identity rather than identity from storage location.
- Do not use a file URI or a synthetic topic URI as identity.
- Preserve Topic membership order and member type, description, threshold, state, and timestamps.
- Apply the intrinsic-shape and extrinsic-brand rules in `docs/architecture/type_identity.md`.
- Use the settled Collection definition.

## Remaining work

- Trace Topic and Feed identity through creation, serialization, reads, directory walks, references, and cache rows.
- Determine how declared IDs are read and validated against storage locations, including existing files whose filename and stored ID disagree.
- Present storage options for Topic, including continued Markdown storage and a shared Atom-based Collection representation, with migration and compatibility costs.
- If multiple Topic formats remain possible, define how reads and path construction discover the format.
- Record the chosen identity and storage model in durable documentation and identify the required migration and implementation changes.

## References

- `docs/architecture/type_identity.md`
- `src/domain.ts`
- `crates/xaga-core/src/domain.rs`
- `crates/xaga-core/src/library.rs`
- `crates/xaga-core/src/md.rs`
- `crates/xaga-core/src/atom.rs`
- `crates/xaga-core/src/persistence/topics.rs`
- `crates/xaga-core/src/persistence/feeds.rs`
