import { expect, it } from "vitest";
import type { ListItem } from "../types";
import { mergeAllItemsAnnotationChange } from "./mergeAllItemsAnnotationChange";

const item = (id: string, createdAt: string, title = id) =>
  ({ id, createdAt, title }) as ListItem;

it("inserts a matching new annotation in descending creation order", () => {
  const newest = item("newest", "2026-03-03T00:00:00Z");
  const oldest = item("oldest", "2026-03-01T00:00:00Z");
  const changed = item("changed", "2026-03-02T00:00:00Z", "Match me");

  expect(
    mergeAllItemsAnnotationChange(
      [newest, oldest],
      changed,
      true,
      "saved",
      "match"
    )
  ).toEqual([newest, changed, oldest]);
});

it("updates an existing annotation without duplicating it", () => {
  const existing = item("annotation", "2026-03-01T00:00:00Z", "Before");
  const changed = item("annotation", "2026-03-01T00:00:00Z", "After");

  expect(
    mergeAllItemsAnnotationChange(
      [existing],
      changed,
      true,
      "saved",
      "missing"
    )
  ).toEqual([changed]);
});

it("does not insert an annotation excluded by the item or text filter", () => {
  const current = [item("existing", "2026-03-01T00:00:00Z")];
  const changed = item("changed", "2026-03-02T00:00:00Z", "Annotation");

  expect(
    mergeAllItemsAnnotationChange(current, changed, true, "unread", "")
  ).toBe(current);
  expect(
    mergeAllItemsAnnotationChange(current, changed, true, "all", "missing")
  ).toBe(current);
});

it("does not insert a new annotation into a feed or Topic list", () => {
  const current = [item("existing", "2026-03-01T00:00:00Z")];
  const changed = item("changed", "2026-03-02T00:00:00Z");

  expect(
    mergeAllItemsAnnotationChange(current, changed, false, "all", "")
  ).toBe(current);
});
