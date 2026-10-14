import { expect, it } from "vitest";
import { imageReferences } from "./imageReferences";

const base = "https://example.org/article";
it("does not treat an empty image source as the article URL", () => {
  expect(imageReferences('<img src="">', base)).toEqual([]);
});
function rewrite(source: string) {
  let bytes = new TextEncoder().encode(source);
  for (const patch of imageReferences(source, base).reverse()) {
    expect(new TextDecoder().decode(bytes.slice(patch.start, patch.end))).toBe(
      patch.original
    );
    const local = new TextEncoder().encode(
      patch.css ? "url(../images/e/hash)" : "../images/e/hash"
    );
    bytes = new Uint8Array([
      ...bytes.slice(0, patch.start),
      ...local,
      ...bytes.slice(patch.end),
    ]);
  }
  return new TextDecoder().decode(bytes);
}
it("changes only Markdown destinations, preserving titles, Unicode and code", () => {
  const source =
    '日本語 ![a cat](<https://example.org/a.png> "extra joke")\n\n`![code](https://example.org/not.png)`';
  expect(rewrite(source)).toBe(
    '日本語 ![a cat](<../images/e/hash> "extra joke")\n\n`![code](https://example.org/not.png)`'
  );
});
it("preserves HTML exactly except the image attribute value", () => {
  const source =
    '<img ALT="A &amp; B" src="/comic?a=1&amp;b=2" title="More content">';
  expect(imageReferences(source, base)[0].url).toBe(
    "https://example.org/comic?a=1&b=2"
  );
  expect(rewrite(source)).toBe(
    '<img ALT="A &amp; B" src="../images/e/hash" title="More content">'
  );
});
it("maps CSS offsets back through HTML entities and preserves unrelated styles", () => {
  const source =
    '<div style="color:red;background-image:url(&quot;/comic.png&quot;)">caption</div>';
  expect(rewrite(source)).toBe(
    '<div style="color:red;background-image:url(../images/e/hash)">caption</div>'
  );
});
it("does not recover local/data references or normal links", () => {
  expect(
    imageReferences(
      "![local](../images/e/hash) [link](https://example.org/a.png) ![inline](data:image/png;base64,AAAA)",
      base
    )
  ).toEqual([]);
});
it("finds CSS image-set string sources", () => {
  expect(
    imageReferences(
      "<div style=\"background-image:image-set('/a.png' 1x, url(/b.png) 2x)\"></div>",
      base
    )
  ).toHaveLength(2);
});
it("finds image references in embedded CSS without treating imports as images", () => {
  const source =
    '<div><style>@import "https://example.org/sheet.css";.comic{background:url(/comic.png)}</style></div>';
  expect(imageReferences(source, base)).toHaveLength(1);
  expect(rewrite(source)).toContain("background:url(../images/e/hash)");
});
it("does not acquire font resources from CSS", () => {
  expect(
    imageReferences(
      "<div><style>@font-face{font-family:test;src:url(https://example.org/font.woff)}</style></div>",
      base
    )
  ).toEqual([]);
});
