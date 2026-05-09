import type { Collection } from "../types";
import sortByTitle from "./sortByTitle";

export default function sortByUnreadCount(a: Collection, b: Collection) {
  return b.unreadCount - a.unreadCount || sortByTitle(a, b);
}
