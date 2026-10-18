/** Favicon locations persisted inside the library. */
export function isLocalFaviconReference(value: string | undefined): boolean {
  return (
    value?.startsWith("../images/") === true ||
    value?.startsWith("../favicons/") === true
  );
}
