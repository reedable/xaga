import { localCss } from "./imageReferences";
import type { ResolveEntryImages } from "./inlineEntryImages";

function fallback(doc: Document, img: Element): Element {
  const text = img.getAttribute("alt")?.trim();
  const title = img.getAttribute("title")?.trim();
  const span = doc.createElement("span");
  span.className = "xaga-unavailable-image";
  span.textContent = text || title || "Image unavailable";
  if (title && title !== text && text) {
    const detail = doc.createElement("span");
    detail.className = "xaga-image-title";
    detail.textContent = title;
    span.append(detail);
  }
  return span;
}

/** Render only local resource bytes, regardless of how a reference arrived. */
export async function prepareEntryImages(
  html: string,
  entryId: string,
  resolve: ResolveEntryImages
): Promise<string> {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const refs = new Set<string>();
  const collect = (value: string) => {
    if (value.startsWith("../images/")) refs.add(value);
    return value;
  };
  doc.querySelectorAll("img").forEach((img) => {
    collect(img.getAttribute("src") ?? "");
  });
  doc
    .querySelectorAll("[style]")
    .forEach((el) => localCss(el.getAttribute("style") ?? "", collect));
  doc
    .querySelectorAll("style")
    .forEach((el) => localCss(el.textContent ?? "", collect, true));
  const paths = [...refs];
  let resolved: (string | null)[] = [];
  if (paths.length) {
    try {
      resolved = await resolve(entryId, paths);
    } catch {
      /* Same unavailable presentation. */
    }
  }
  const byRef = new Map(paths.map((path, i) => [path, resolved[i]]));
  const local = (value: string): string | null => {
    if (/^data:image\//i.test(value)) return value;
    return byRef.get(value) ?? null;
  };
  // Responsive candidates must never bypass the resolved src.
  doc.querySelectorAll("source, link").forEach((el) => el.remove());
  doc
    .querySelectorAll("[srcset]")
    .forEach((el) => el.removeAttribute("srcset"));
  doc.querySelectorAll("img").forEach((img) => {
    const value = local(img.getAttribute("src") ?? "");
    if (value) img.setAttribute("src", value);
    else img.replaceWith(fallback(doc, img));
  });
  doc.querySelectorAll("[style]").forEach((el) => {
    let unavailable = false;
    const style = localCss(el.getAttribute("style") ?? "", (value) => {
      const result = local(value);
      if (!result) unavailable = true;
      return result;
    });
    el.setAttribute("style", style);
    if (unavailable && !el.textContent?.trim()) {
      el.append(fallback(doc, el));
    }
  });
  doc.querySelectorAll("style").forEach((el) => {
    el.textContent = localCss(el.textContent ?? "", local, true);
  });
  return doc.body.innerHTML;
}
