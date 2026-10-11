// TODO: Move this back into entry.worker.ts (Web Worker) once the
// `ReferenceError: Can't find variable: document` in the remark-stringify
// bundle chunk is resolved. Some package in the unified/rehype/remark pipeline
// accesses the global `document` inside the worker context where it doesn't exist.

import { Readability } from "@mozilla/readability";
import DOMPurify from "dompurify";
import rehypeParse from "rehype-parse";
import rehypeRemark from "rehype-remark";
import remarkGfm from "remark-gfm";
import remarkStringify from "remark-stringify";
import { unified } from "unified";
import type { ArticleDiagnosticStep } from "../types";

const HEADING_SELECTORS = [
  ...[1, 2, 3, 4, 5, 6].map(
    (n) => `h${n}, [role="heading"][aria-level="${n}"]`
  ),
  '[role="heading"]:not([aria-level])',
];

function extractHeadingTitle(doc: Document): string {
  for (const selector of HEADING_SELECTORS) {
    const text = doc.querySelector(selector)?.textContent?.trim();
    if (text) return text;
  }
  return "Untitled";
}

function extractFavicon(doc: Document, url: string): string | null {
  const links = Array.from(
    doc.querySelectorAll<HTMLLinkElement>('link[rel~="icon"]')
  );
  if (links.length === 0) return null;

  const preferred =
    links.find((l) => l.getAttribute("sizes") === "32x32") ??
    links.find((l) => l.getAttribute("sizes") === "16x16") ??
    links[0];

  const href = preferred.getAttribute("href");
  if (!href) return null;

  try {
    return new URL(href, url).href;
  } catch {
    return null;
  }
}

/**
 * The favicon the loader resolved and fetched, when it managed to. Supplied by
 * the caller rather than derived here: the loader webview is the only context
 * permitted to fetch remote content, so the bytes have to come from it.
 */
export type LoaderFavicon = { url: string; dataUri: string };

export async function processEntry(
  url: string,
  html: string,
  loaderFavicon?: LoaderFavicon | null,
  phase?: (step: ArticleDiagnosticStep) => void
): Promise<{
  title: string;
  markdown: string;
  favicon: string | null;
  /** `url → data:` for the images the loader fetched that survived extraction.
   * The body carries the remote URLs; Rust hashes them, writes files from these
   * bytes, and uses local URLs only for successfully persisted body images. */
  imageData: Record<string, string>;
}> {
  phase?.("parse");
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, "text/html");

  const base = doc.createElement("base");
  base.href = url;
  doc.head.prepend(base);

  // Prefer the loader's pair: that URL is the one whose bytes we actually hold,
  // so it is the one the backend must hash. Without it we fall back to reading
  // the declared `<link rel~=icon>`, which yields a URL and no bytes — the entry
  // then stores a remote favicon URL, as it did before this path existed.
  const favicon = loaderFavicon?.url ?? extractFavicon(doc, url);

  // Pre-Readability: the loader inlined each fetched image as a `data:` src and
  // stashed the remote URL in `data-xaga-url`. Capture that pairing now, then
  // strip the stash — otherwise Readability treats the `data:`-plus-URL pattern
  // as a lazy-load hint and overwrites the src with the URL, destroying the
  // fetched bytes.
  const dataToUrl = new Map<string, string>();

  for (const img of Array.from(doc.querySelectorAll("img"))) {
    const stashed = img.getAttribute("data-xaga-url");
    if (!stashed) continue;

    const src = img.getAttribute("src");
    if (src && src.startsWith("data:")) dataToUrl.set(src, stashed);

    img.removeAttribute("data-xaga-url");
  }

  phase?.("readability");
  const entry = new Readability(doc).parse();

  if (!entry) {
    throw new Error("Readability could not extract an entry");
  }

  // Post-Readability: restore each surviving fetched image's src to its remote
  // URL (so the body carries URLs, which Rust rewrites to `../images/<hash>`),
  // and collect `url → data:` for those kept images.
  phase?.("images");
  const contentDoc = parser.parseFromString(entry.content ?? "", "text/html");
  const imageData: Record<string, string> = {};

  // The favicon rides in the same `url → data:` map as the body images, so it
  // reaches the backend without widening the save API.
  if (loaderFavicon) imageData[loaderFavicon.url] = loaderFavicon.dataUri;

  for (const img of Array.from(contentDoc.querySelectorAll("img"))) {
    const src = img.getAttribute("src");
    if (!src || !src.startsWith("data:")) continue;

    const remote = dataToUrl.get(src);
    if (remote) {
      img.setAttribute("src", remote);
      imageData[remote] = src;
    }

    // A `data:` with no known URL is an original page data-URI; leave it — Rust's
    // extraction drops stray `data:` images so no base64 reaches the stored body.
  }

  const restoredContent = contentDoc.body.innerHTML;

  // Sanitize the (URL-restored) content before anything else consumes it — the
  // explicit security boundary for article bodies. See
  // docs/architecture/security/handling_remote_content.md, "Processing," for why
  // this must not be left to the downstream HTML round-trip.
  //
  // ASSUMPTION: neither DOMParser (above) nor Readability ever executes scripts
  // or fires event handlers — both operate on an inert, non-rendered document, so
  // sanitizing the *output* here is sufficient. If that ever stops holding, the
  // untrusted HTML would need to be sanitized *before* it is parsed instead.
  phase?.("sanitize");
  const sanitizedContent = DOMPurify.sanitize(restoredContent);

  phase?.("markdown");
  const file = await unified()
    .use(rehypeParse, { fragment: true })
    .use(rehypeRemark)
    .use(remarkGfm)
    .use(remarkStringify)
    .process(sanitizedContent);

  const readabilityTitle = entry.title?.trim();
  const title = readabilityTitle || extractHeadingTitle(doc);

  return { title, markdown: String(file), favicon, imageData };
}
