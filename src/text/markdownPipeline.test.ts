import { describe, expect, it } from "vitest";
import { markdownToHtml } from "./markdownPipeline";

// Pins the render-path security control: untrusted `.md`/feed HTML is parsed
// (rehype-raw) so it renders, then sanitized (DOMPurify) so nothing dangerous
// survives. A regression in either — a dropped DOMPurify call, a dependency
// default change — fails here.
describe("markdownToHtml", () => {
  it("renders markdown", async () => {
    expect(await markdownToHtml("**bold**")).toContain("<strong>bold</strong>");
  });

  it("preserves safe embedded HTML (the feed-summary regression)", async () => {
    const html = await markdownToHtml(
      '<p>Hello <a href="https://x.test/">link</a></p>'
    );
    expect(html).toContain("<p>");
    expect(html).toContain("Hello");
    expect(html).toContain('href="https://x.test/"');
  });

  it("strips <script> tags", async () => {
    const html = await markdownToHtml("<p>ok</p><script>alert(1)</script>");
    expect(html).toContain("ok");
    expect(html).not.toContain("<script");
  });

  it("strips event-handler attributes", async () => {
    const html = await markdownToHtml('<img src="x" onerror="alert(1)">');
    expect(html.toLowerCase()).not.toContain("onerror");
  });

  it("strips javascript: hrefs", async () => {
    const html = await markdownToHtml('<a href="javascript:alert(1)">x</a>');
    expect(html.toLowerCase()).not.toContain("javascript:");
  });
});

// Characterization of the remote-fetch channel at the render boundary for
// untrusted stored content. These pin what the sanitizer does TODAY, including
// the parts that are the finding rather than the fix: a `https:` image
// surviving here is what lets rendered content call home at read time. If
// rendered content is made inert (bytes inlined as data URLs, `https:` dropped
// from `img-src`), the "passes through" cases below are the ones expected to
// flip, deliberately.
describe("markdownToHtml — remote-fetch surface", () => {
  it("passes remote https: image sources through (the open channel)", async () => {
    const html = await markdownToHtml('<img src="https://tracker.test/p.gif">');
    expect(html).toContain('src="https://tracker.test/p.gif"');
  });

  it("preserves data: image sources (inert content is renderable)", async () => {
    const dataUri =
      "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
    const html = await markdownToHtml(`<img src="${dataUri}">`);
    expect(html).toContain(dataUri);
  });

  it("preserves data: images written as markdown image syntax", async () => {
    const dataUri =
      "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
    const html = await markdownToHtml(`![alt](${dataUri})`);
    expect(html).toContain(dataUri);
    expect(html).toContain("<img");
  });

  it("records whether style attributes survive (background-image channel)", async () => {
    const html = await markdownToHtml(
      '<div style="background-image: url(https://tracker.test/bg.png)">x</div>'
    );
    // The compliance doc asserts this is a second copy of the image channel.
    // Whichever way this lands, it is now pinned rather than assumed.
    expect(html).toMatchInlineSnapshot(`"<div style="background-image: url(https://tracker.test/bg.png)">x</div>"`);
  });

  it("records whether <iframe> survives (frame-src https: reachability)", async () => {
    const html = await markdownToHtml(
      '<iframe src="https://embed.test/v"></iframe>'
    );
    expect(html).toMatchInlineSnapshot(`""`);
  });

  // The entry-image feature stores `![alt](../images/<id>/<hash>)` in the body
  // and rewrites it to a data: URI at render (inlineEntryImages), AFTER this
  // sanitizer. If DOMPurify ever stripped the relative src, every stored image
  // would silently stop resolving. This pins that the relative ref survives.
  it("preserves the relative ../images src through sanitization", async () => {
    const relative = await markdownToHtml(
      "![local](../images/a3f2c1d0e9b8/4f2a91c3e0b7)"
    );
    expect(relative).toContain('src="../images/a3f2c1d0e9b8/4f2a91c3e0b7"');
  });
});
