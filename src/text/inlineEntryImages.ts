// Render-time step: replace an entry's stored `../images/…` references with the
// `data:` URIs their bytes decode to, so the reader shows locally-stored images
// without any network request. Runs AFTER markdownToHtml (so it operates on
// sanitized HTML) and BEFORE buildEntryHtml (so the reader's inline script — and
// therefore the CSP script hash — is untouched).
//
// Backwards compatibility is the whole point of the short-circuit below: an
// entry saved before this feature has only `https:`/`http:` srcs and no
// `../images/` reference, so it returns the input string unchanged and never
// calls `resolve`. Nothing about how existing entries render changes.

/**
 * Resolves the given entry's image references to `data:` URIs (or `null` where a
 * reference does not resolve), index-aligned with `refs`. Injected so this
 * module is pure and testable without the Tauri platform layer.
 */
export type ResolveEntryImages = (
  entryId: string,
  refs: string[]
) => Promise<(string | null)[]>;

const LOCAL_PREFIX = "../images/";

/**
 * Rewrites `<img src="../images/…">` references in already-sanitized reader HTML
 * to `data:` URIs. Any other `src` — `https:`, `http:`, `data:`, protocol- or
 * root-relative, anything unrecognized — is left exactly as-is and never sent to
 * `resolve`. If resolution throws or a reference does not resolve, the original
 * `src` is left in place; in the sandboxed blob iframe a relative `src` fetches
 * nothing, so the failure mode is a broken image, never a network leak.
 */
export async function inlineEntryImages(
  html: string,
  entryId: string,
  resolve: ResolveEntryImages
): Promise<string> {
  // Fast path and the compat guarantee: no local reference means nothing to do,
  // and the input is returned unchanged without parsing or calling resolve.
  if (!html.includes(LOCAL_PREFIX)) return html;

  let doc: Document;
  try {
    doc = new DOMParser().parseFromString(html, "text/html");
  } catch {
    return html;
  }

  // Collect the raw attribute values — getAttribute, never .src, which would
  // resolve against the document base and erase the `../images/` distinction.
  const imgs = Array.from(doc.querySelectorAll("img"));
  const refs: string[] = [];
  for (const img of imgs) {
    const src = img.getAttribute("src");
    if (src && src.startsWith(LOCAL_PREFIX) && !refs.includes(src)) {
      refs.push(src);
    }
  }
  if (refs.length === 0) return html;

  let resolved: (string | null)[];
  try {
    resolved = await resolve(entryId, refs);
  } catch {
    // Resolution unavailable: leave every reference untouched.
    return html;
  }

  const byRef = new Map<string, string>();
  refs.forEach((ref, i) => {
    const dataUri = resolved[i];
    if (typeof dataUri === "string") byRef.set(ref, dataUri);
  });
  if (byRef.size === 0) return html;

  for (const img of imgs) {
    const src = img.getAttribute("src");
    if (src) {
      const dataUri = byRef.get(src);
      if (dataUri) img.setAttribute("src", dataUri);
    }
  }

  return doc.body.innerHTML;
}
