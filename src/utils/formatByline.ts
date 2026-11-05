import { parse } from "tldts";
import { isEntry, type ManagedEntity } from "../types";
import formatDate from "./formatDate";

export default function formatByline(entity: ManagedEntity) {
  const displayDate = formatDate(entity);

  if (isEntry(entity)) {
    const displayHost = formatHost(entity);
    return [displayDate, displayHost].filter(Boolean).join(" • ");
  }

  return displayDate;
}

function formatHost(entity: ManagedEntity) {
  if (isEntry(entity)) {
    return entity.url ? (parse(entity.url).domain ?? null) : null;
  }

  return null;
}
