import { isEntry, type Item } from "../types";

const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

export default function formatDate(item: Item): string {
  if (isEntry(item)) {
    const date = stringToDate(item.updatedAt, item.publishedAt, item.createdAt);
    return _formatDate(date);
  }

  const date = stringToDate(item.updatedAt, item.createdAt);
  return _formatDate(date);
}

function _formatDate(date: Date | null): string {
  if (!date) return "";
  if (isNaN(date.getTime())) return "";

  const diffMs = date.getTime() - Date.now();
  const diffM = Math.round(diffMs / MINUTE);
  if (Math.abs(diffM) < 60) return rtf.format(diffM, "minute");

  const diffH = Math.round(diffMs / HOUR);
  if (Math.abs(diffH) < 24) return rtf.format(diffH, "hour");

  const diffD = Math.round(diffMs / DAY);
  if (Math.abs(diffD) < 3) return rtf.format(diffD, "day");

  const sameYear = date.getFullYear() === new Date().getFullYear();

  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    ...(sameYear ? {} : { year: "numeric" }),
    hour: "numeric",
    minute: "2-digit",
  });
}

// Returns the first dateString that is value and not UNIX epoch
function stringToDate(...dateStrings: (string | undefined)[]): Date | null {
  const ts = dateStrings.reduce((r: number, dateString) => {
    if (!dateString) return r;
    const date = new Date(dateString);
    return date.getTime() || r;
  }, 0);

  return ts ? new Date(ts) : null;
}
