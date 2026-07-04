# ItemList does not reload after topic membership change (pending)

Adding or removing a manual item via `TopicMenuButton` does not reload the item list
(`src/components/list/ItemList.tsx`); the change is not reflected until some other reload
occurs.
