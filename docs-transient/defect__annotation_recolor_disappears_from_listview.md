# Annotation color change makes the annotation disappear from the ListView (pending)

A color change replaces the annotation under a new id — the "Annotation color change is
delete/create in one atomic write" instance in `docs/architecture/crash_consistency.md`. The
consequence in the ListView: the annotation the user selected in the item view is gone when
the color changes. The `replace_annotation` command emits `annotation:removed` for the old id
(consumers evict it), and the new id arrives as a *new* annotation through the live
`sync_annotations` event — but the list does not add newly created annotations
(`docs-transient/sprint/defect__itemlist_annotation_added_not_listed.md`), so the recolored annotation
vanishes from the list until a reload, and any selection anchored to the old id is dropped.

This is a UX gap, not a data defect: the `.md` and LanceDB are correct; only the live list
state loses the item. A UX improvement is needed so that, from the user's point of view, a
recolor reads as the same annotation changing color, not as a removal — e.g. the list follows
the replacement (evict + insert as one visible operation) and selection moves from the old id
to the new one. Mechanism open; depends on how the list add path
(`docs-transient/sprint/defect__itemlist_annotation_added_not_listed.md`) is resolved.
