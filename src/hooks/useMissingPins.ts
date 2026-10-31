import { useMemo } from "react";
import { usePins } from "../providers/PinsContext";
import type { Entry, Feed, Topic } from "../types";

/** Shared target resolution for sidebar and responsive navigation. */
export function useMissingPins(
  entries: Entry[],
  feeds: Feed[],
  topics: Topic[]
) {
  const { pins } = usePins();
  return useMemo(
    () =>
      pins.filter((target) => {
        const items =
          target.type === "entry"
            ? entries
            : target.type === "feed"
              ? feeds
              : topics;
        return !items.some((item) => item.id === target.id);
      }),
    [pins, entries, feeds, topics]
  );
}
