import { expect, it } from "vitest";
import type { ListItem } from "../types";
import { mergeTopicAddition } from "./mergeTopicAddition";

const item = (id: string) => ({ id }) as ListItem;

it("inserts an addition in authoritative order while retaining stale removals", () => {
  const stale = item("removed");
  const current = [item("first"), stale, item("last")];
  const authoritative = [item("first"), item("added"), item("last")];

  expect(mergeTopicAddition(current, authoritative, "added")).toEqual([
    current[0],
    stale,
    authoritative[1],
    current[2],
  ]);
});

it("does not add duplicates or items excluded by the active filter", () => {
  const current = [item("present")];
  expect(mergeTopicAddition(current, [item("present")], "present")).toBe(
    current
  );
  expect(mergeTopicAddition(current, [], "filtered-out")).toBe(current);
});

it("inserts a replacement after the old annotation has been evicted", () => {
  const current = [item("before"), item("after")];
  const authoritative = [item("before"), item("recolored"), item("after")];

  expect(mergeTopicAddition(current, authoritative, "recolored")).toEqual([
    current[0],
    authoritative[1],
    current[1],
  ]);
});
