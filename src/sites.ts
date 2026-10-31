import type { Site } from "./types";

/** A known Site identity remains navigable when its derived membership is empty. */
export function emptySite(hostname: string): Site {
  return {
    id: hostname,
    hostname,
    title: hostname,
    entryCount: 0,
    unreadCount: 0,
  };
}
