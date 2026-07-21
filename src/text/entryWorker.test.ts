import { describe, expect, it } from "vitest";
import { processEntry } from "./entryWorker";

// `processEntry` is the ingestion boundary: loader-captured article HTML is
// sanitized and converted to markdown, and markdown is what gets persisted
// (`save_article` takes `markdown`, not HTML). Anything an upstream step writes
// into the HTML — e.g. images inlined as data URLs inside the loader webview —
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
  it("carries data: image sources through to markdown", async () => {
    const { markdown } = await processEntry(
      "https://site.test/a",
      articleHtml(`<img src="${DATA_URI}" alt="inline">`)
    );
    expect(markdown).toContain(DATA_URI);
  });

  it("carries remote https: image sources through to markdown", async () => {
    const { markdown } = await processEntry(
      "https://site.test/a",
      articleHtml('<img src="https://tracker.test/p.gif" alt="remote">')
    );
    expect(markdown).toContain("https://tracker.test/p.gif");
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
