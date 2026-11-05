import { expect, it } from "vitest";
import type { ListEntity } from "../types";
import { mergeAllEntitiesAnnotationChange } from "./mergeAllEntitiesAnnotationChange";

const entity = (id: string, createdAt: string, title = id) =>
  ({ id, createdAt, title }) as ListEntity;

it("inserts a matching new annotation in descending creation order", () => {
  const newest = entity("newest", "2026-03-03T00:00:00Z");
  const oldest = entity("oldest", "2026-03-01T00:00:00Z");
  const changed = entity("changed", "2026-03-02T00:00:00Z", "Match me");

  expect(
    mergeAllEntitiesAnnotationChange(
      [newest, oldest],
      changed,
      true,
      "saved",
      "match"
    )
  ).toEqual([newest, changed, oldest]);
});

it("updates an existing annotation without duplicating it", () => {
  const existing = entity("annotation", "2026-03-01T00:00:00Z", "Before");
  const changed = entity("annotation", "2026-03-01T00:00:00Z", "After");

  expect(
    mergeAllEntitiesAnnotationChange(
      [existing],
      changed,
      true,
      "saved",
      "missing"
    )
  ).toEqual([changed]);
});

it("does not insert an annotation excluded by the item or text filter", () => {
  const current = [entity("existing", "2026-03-01T00:00:00Z")];
  const changed = entity("changed", "2026-03-02T00:00:00Z", "Annotation");

  expect(
    mergeAllEntitiesAnnotationChange(current, changed, true, "unread", "")
  ).toBe(current);
  expect(
    mergeAllEntitiesAnnotationChange(current, changed, true, "all", "missing")
  ).toBe(current);
});

it("does not insert a new annotation into a feed or Topic list", () => {
  const current = [entity("existing", "2026-03-01T00:00:00Z")];
  const changed = entity("changed", "2026-03-02T00:00:00Z");

  expect(
    mergeAllEntitiesAnnotationChange(current, changed, false, "all", "")
  ).toBe(current);
});
