import { describe, expect, it } from "vitest";
import { processEntry } from "./entryWorker";

// `processEntry` is the ingestion boundary: loader-captured article HTML is
// sanitized and converted to markdown, and markdown is what gets persisted
// (`save_article` takes `markdown`, not HTML). Anything an upstream step writes
// into the HTML — e.g. images fetched as data URLs inside the loader webview —
// only reaches the reader if it survives this conversion.

/** Readability needs a substantial body before it will extract anything. */
function articleHtml(body: string): string {
  const filler = "<p>".concat(
    "Sentences of ordinary article prose that give Readability enough text to score this container as the main content. ".repeat(
      12
    ),
    "</p>"
  );
  return `<!doctype html><html><head><title>T</title></head><body>
    <article>${filler}${body}${filler}</article>
  </body></html>`;
}

const DATA_URI =
  "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

describe("processEntry — image URLs across the HTML→markdown boundary", () => {
  it("restores a fetched image's URL to the body and returns its bytes", async () => {
    // The loader fetched this image: data: src + the stashed remote URL.
    const url = "https://cdn.test/photo.jpg";
    const { markdown, imageData } = await processEntry(
      "https://site.test/a",
      articleHtml(`<img src="${DATA_URI}" data-xaga-url="${url}" alt="photo">`)
    );
    // Body carries the URL (Rust hashes it to a local ref), not the data: bytes.
    expect(markdown).toContain(url);
    expect(markdown).not.toContain(DATA_URI);
    // The bytes are returned to the caller, keyed by URL.
    expect(imageData[url]).toBe(DATA_URI);
  });

  it("keeps a non-fetched remote image as a URL with no bytes", async () => {
    // A CORS-missed image: still remote https, no data: and no stash.
    const { markdown, imageData } = await processEntry(
      "https://site.test/a",
      articleHtml('<img src="https://tracker.test/p.gif" alt="remote">')
    );
    expect(markdown).toContain("https://tracker.test/p.gif");
    expect(Object.keys(imageData)).toHaveLength(0);
  });

  it("returns no bytes for a stray data: image with no stashed URL", async () => {
    // An original page data-URI (not loader-fetched): no URL to pair, so no
    // entry in imageData. Rust extraction drops it from the stored body.
    const { markdown, imageData } = await processEntry(
      "https://site.test/a",
      articleHtml(`<img src="${DATA_URI}" alt="icon">`)
    );
    expect(markdown).toContain(DATA_URI);
    expect(Object.keys(imageData)).toHaveLength(0);
  });

  it("records whether style background-image survives ingestion", async () => {
    const { markdown } = await processEntry(
      "https://site.test/a",
      articleHtml(
        '<div style="background-image: url(https://tracker.test/bg.png)">x</div>'
      )
    );
    // markdownToHtml preserves this on the feed-summary path; whether the
    // HTML→markdown conversion also carries it decides if the article path
    // shares that channel.
    expect(markdown.includes("tracker.test/bg.png")).toMatchInlineSnapshot(`false`);
  });

  it("strips <iframe> at ingestion", async () => {
    const { markdown } = await processEntry(
      "https://site.test/a",
      articleHtml('<iframe src="https://embed.test/v"></iframe>')
    );
    expect(markdown).not.toContain("embed.test");
  });
});
