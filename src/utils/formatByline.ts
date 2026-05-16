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
    return item.url
      ? new URL(item.url).hostname.split(".").slice(-2).join(".")
      : null;
  }

  return null;
}
