import { parse } from "tldts";
import { isEntry, type Item } from "../types";
import formatDate from "./formatDate";

export default function formatByline(item: Item) {
  const displayDate = formatDate(item);

  if (isEntry(item)) {
    const displayHost = formatHost(item);
    return [displayDate, displayHost].filter(Boolean).join(" • ");
  }

  return displayDate;
}

function formatHost(item: Item) {
  if (isEntry(item)) {
    return item.url ? (parse(item.url).domain ?? null) : null;
  }

  return null;
}
