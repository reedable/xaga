import type { Site } from "./domain";

/** A known Site identity remains navigable when its derived membership is empty. */
export function emptySite(hostname: string): Site {
  return {
    __kind: "site",
    id: hostname,
    title: hostname,
    entryCount: 0,
    unreadCount: 0,
  };
}
