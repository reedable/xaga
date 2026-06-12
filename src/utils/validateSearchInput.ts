// Single validation chokepoint for the free-form search query. Non-strings
// (e.g. an absent URL param) yield undefined, which the UI treats as "search
// closed". A string — including "" — yields a trimmed, length-capped query,
// where "" means "search open but empty".
const MAX_SEARCH_LENGTH = 512;

export default function validateSearchInput(value?: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  return value.trim().slice(0, MAX_SEARCH_LENGTH);
}
