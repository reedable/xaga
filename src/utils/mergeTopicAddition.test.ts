import { expect, it } from "vitest";
import type { ListEntity } from "../types";
import { mergeTopicAddition } from "./mergeTopicAddition";

const entity = (id: string) => ({ id }) as ListEntity;

it("inserts an addition in authoritative order while retaining stale removals", () => {
  const stale = entity("removed");
  const current = [entity("first"), stale, entity("last")];
  const authoritative = [entity("first"), entity("added"), entity("last")];

  expect(mergeTopicAddition(current, authoritative, "added")).toEqual([
    current[0],
    stale,
    authoritative[1],
    current[2],
  ]);
});

it("does not add duplicates or items excluded by the active filter", () => {
  const current = [entity("present")];
  expect(mergeTopicAddition(current, [entity("present")], "present")).toBe(
    current
  );
  expect(mergeTopicAddition(current, [], "filtered-out")).toBe(current);
});

it("inserts a replacement after the old annotation has been evicted", () => {
  const current = [entity("before"), entity("after")];
  const authoritative = [entity("before"), entity("recolored"), entity("after")];

  expect(mergeTopicAddition(current, authoritative, "recolored")).toEqual([
    current[0],
    authoritative[1],
    current[1],
  ]);
});
