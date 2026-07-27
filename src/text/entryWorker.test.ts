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

// The favicon is fetched in the loader webview, never here — the main window
// must not issue that request. `processEntry` only pairs the loader's URL with
// its bytes and puts them where `save_article` already looks.
describe("processEntry — favicon", () => {
  it("prefers the loader's pair and carries its bytes in imageData", async () => {
    const loaderFavicon = {
      url: "https://cdn.test/icon.png",
      dataUri: DATA_URI,
    };
    const { favicon, imageData } = await processEntry(
      "https://site.test/a",
      articleHtml("<p>text</p>").replace(
        "<head>",
        '<head><link rel="icon" href="/declared.ico">'
      ),
      loaderFavicon
    );
    // The loader's URL wins: it is the one whose bytes we hold, so it is the one
    // the backend must hash.
    expect(favicon).toBe(loaderFavicon.url);
    expect(imageData[loaderFavicon.url]).toBe(DATA_URI);
  });

  it("falls back to the declared link with no bytes when the loader sent none", async () => {
    const { favicon, imageData } = await processEntry(
      "https://site.test/a",
      articleHtml("<p>text</p>").replace(
        "<head>",
        '<head><link rel="icon" href="/declared.ico">'
      )
    );
    expect(favicon).toBe("https://site.test/declared.ico");
    expect(Object.keys(imageData)).toHaveLength(0);
  });
});

// `extract_images` (Rust) rewrites exactly one shape: a markdown image
// `![alt](url)`. So the security property of this boundary is that no construct
// capable of issuing a request survives it in any *other* shape — no raw HTML
// tag, no CSS `url()`, no `srcset`. A remote URL left in markdown link syntax is
// fine: a link is not fetched.
describe("processEntry — no fetchable markup survives ingestion", () => {
  const CONSTRUCTS: [string, string][] = [
    ["img", '<img src="https://t.test/a.png">'],
    ["img srcset", '<img src="https://t.test/a.png" srcset="https://t.test/2x.png 2x">'],
    [
      "picture/source",
      '<picture><source srcset="https://t.test/s.webp"><img src="https://t.test/a.png"></picture>',
    ],
    ["video poster", '<video poster="https://t.test/p.jpg" src="https://t.test/v.mp4"></video>'],
    ["audio", '<audio src="https://t.test/a.mp3"></audio>'],
    ["track", '<video><track src="https://t.test/t.vtt"></video>'],
    ["object", '<object data="https://t.test/o.bin"></object>'],
    ["embed", '<embed src="https://t.test/e">'],
    ["input type=image", '<input type="image" src="https://t.test/i.png">'],
    ["svg image", '<svg><image href="https://t.test/i.png"/></svg>'],
    ["link stylesheet", '<link rel="stylesheet" href="https://t.test/s.css">'],
    ["style block", "<style>body{background:url(https://t.test/bg.png)}</style>"],
    ["inline style", '<div style="background-image:url(https://t.test/bg.png)">x</div>'],
  ];

  it.each(CONSTRUCTS)("%s leaves no fetchable markup", async (_name, body) => {
    const { markdown } = await processEntry(
      "https://site.test/a",
      articleHtml(body)
    );
    expect(markdown).not.toMatch(
      /<(img|iframe|object|embed|source|picture|video|audio|input|svg|image|link|style|track)\b/i
    );
    expect(markdown).not.toMatch(/url\(/i);
    expect(markdown).not.toMatch(/srcset/i);
  });
});

// Rust's `is_remote` matches an `http`/`https` prefix, so a relative or
// protocol-relative `src` that reached it unresolved would be left in the body
// verbatim and fetch at render. Readability resolves against the injected
// `<base>` before that, and this pins it.
describe("processEntry — src shapes arrive absolute", () => {
  it.each([
    ["protocol-relative", "//cdn.test/a.png", "https://cdn.test/a.png"],
    ["root-relative", "/a.png", "https://site.test/a.png"],
    ["path-relative", "a.png", "https://site.test/a.png"],
    ["uppercase scheme", "HTTPS://CDN.TEST/a.png", "https://cdn.test/a.png"],
  ])("resolves a %s src", async (_name, src, expected) => {
    const { markdown } = await processEntry(
      "https://site.test/a",
      articleHtml(`<img src="${src}">`)
    );
    expect(markdown).toContain(`![](${expected})`);
  });
});
