import type { Countable } from "../types";
import sortByTitle from "./sortByTitle";

export default function sortByUnreadCount(a: Countable, b: Countable) {
  return b.unreadCount - a.unreadCount || sortByTitle(a, b);
}
