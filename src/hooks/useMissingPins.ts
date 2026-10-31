import { useMemo, useContext } from "react";
import SitesContext from "../providers/SitesContext";
import { usePins } from "../providers/PinsContext";
import type { Entry, Feed, Topic } from "../types";

/** Shared target resolution for sidebar and responsive navigation. */
export function useMissingPins(
  entries: Entry[],
  feeds: Feed[],
  topics: Topic[]
) {
  const { pins } = usePins();
  const sites = useContext(SitesContext);
  return useMemo(
    () =>
      pins.filter((target) => {
        // A pinned Site with no current Entries remains a valid, empty Site.
        if (target.type === "site") return sites?.hasLoaded === false;
        const items =
          target.type === "entry"
            ? entries
            : target.type === "feed"
              ? feeds
              : topics;
        return !items.some((item) => item.id === target.id);
      }),
    [pins, entries, feeds, topics, sites?.hasLoaded]
  );
}
