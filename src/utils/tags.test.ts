import { expect, it } from "vitest";
import { resolveMergedTagIds, retainTagMerges } from "./tags";

it("retains every losing identity when the same survivor wins successive merges", () => {
  const merge = (
    sourceId: string,
    destinationId: string,
    survivorId: string
  ) => ({ sourceId, destinationId, survivorId, destinationName: "Name" });
  let history = retainTagMerges([], [merge("a", "b", "a")]);
  history = retainTagMerges(history, [merge("a", "c", "a")]);
  history = retainTagMerges(history, [merge("a", "d", "d")]);
  expect(
    resolveMergedTagIds(["b", "c", "a", "d", "unrelated"], history)
  ).toEqual(["d", "unrelated"]);
});
