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
