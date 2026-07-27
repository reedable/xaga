# New annotations not added to the item list (pending)

`onAnnotationChanged` in `src/components/list/ItemList.tsx` does not add newly created annotations to the list; existing-annotation changes propagate, new ones do not appear until a reload.
