// TODO: Move this back into entry.worker.ts (Web Worker) once the
// `ReferenceError: Can't find variable: document` in the remark-stringify
// bundle chunk is resolved. Some package in the unified/rehype/remark pipeline
// accesses the global `document` inside the worker context where it doesn't exist.

import { Readability } from "@mozilla/readability";
import { unified } from "unified";
import rehypeParse from "rehype-parse";
import rehypeRemark from "rehype-remark";
import remarkGfm from "remark-gfm";
import remarkStringify from "remark-stringify";

const HEADING_SELECTORS = [
  ...[1, 2, 3, 4, 5, 6].map(
    (n) => `h${n}, [role="heading"][aria-level="${n}"]`,
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
  const links = Array.from(doc.querySelectorAll<HTMLLinkElement>('link[rel~="icon"]'));
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

export async function processEntry(
  url: string,
  html: string,
): Promise<{ title: string; markdown: string; favicon: string | null }> {
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, "text/html");

  const base = doc.createElement("base");
  base.href = url;
  doc.head.prepend(base);

  const favicon = extractFavicon(doc, url);

  const entry = new Readability(doc).parse();
  if (!entry) {
    throw new Error("Readability could not extract an entry");
  }

  const file = await unified()
    .use(rehypeParse, { fragment: true })
    .use(rehypeRemark)
    .use(remarkGfm)
    .use(remarkStringify)
    .process(entry.content ?? "");

  const readabilityTitle = entry.title?.trim();
  const title = readabilityTitle || extractHeadingTitle(doc);
  return { title, markdown: String(file), favicon };
}
