import { expect, it, vi } from "vitest";
import { prepareEntryImages } from "./prepareEntryImages";
const data = "data:image/png;base64,AAAA";
it("uses the same useful fallback for remote and missing local images", async () => {
  const resolve = vi.fn().mockResolvedValue([null]);
  const html = await prepareEntryImages(
    '<img src="https://example.org/a" alt="Comic" title="Joke"><img src="../images/e/missing" alt="Comic" title="Joke">',
    "e",
    resolve
  );
  const doc = new DOMParser().parseFromString(html, "text/html");
  expect(doc.querySelectorAll("img")).toHaveLength(0);
  expect(doc.querySelectorAll(".xaga-unavailable-image")).toHaveLength(2);
  expect(doc.querySelectorAll(".xaga-image-title")[0].textContent).toBe("Joke");
  expect(resolve).toHaveBeenCalledWith("e", ["../images/e/missing"]);
});
it("resolves local HTML and CSS images and removes remote candidates", async () => {
  const html = await prepareEntryImages(
    '<img src="../images/e/hash" srcset="https://remote/a 2x"><div style="background-image:url(../images/e/hash)"></div><div style="color:red;background-image:url(https://remote/b)">Text</div>',
    "e",
    async () => [data]
  );
  expect(html).toContain(`src="${data}"`);
  expect(html).toContain(`url(${data})`);
  expect(html).not.toContain("https://remote");
  expect(html).toContain("color:red");
});
it("provides a compact fallback without alt text and handles resolver failure", async () => {
  const html = await prepareEntryImages(
    '<img src="../images/e/missing"><div style="background:url(https://remote/a)"></div>',
    "e",
    async () => {
      throw new Error("offline");
    }
  );
  expect(html.match(/Image unavailable/g)).toHaveLength(2);
  expect(html).not.toContain("url(");
});
it("localizes embedded CSS and excludes external stylesheet imports", async () => {
  const html = await prepareEntryImages(
    "<div><style>@import url(https://remote/style.css);.comic{background:url(../images/e/hash)}</style></div>",
    "e",
    async () => [data]
  );
  expect(html).toContain(`url(${data})`);
  expect(html).not.toContain("@import");
  expect(html).not.toContain("https://");
});
