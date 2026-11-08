import { expect, it, vi } from "vitest";
import { imageReferences } from "./imageReferences";
import { markdownToHtml } from "./markdownPipeline";
import { prepareEntryImages } from "./prepareEntryImages";

const cases = [
  [
    "density candidates only",
    '<img alt="Density photo" srcset="small.png 1x, large.png 2x">',
    [],
  ],
  [
    "width candidates only",
    '<img alt="Width photo" srcset="small.png 480w, large.png 1200w" sizes="(max-width: 40rem) 100vw, 40rem">',
    [],
  ],
  [
    "picture media alternatives",
    '<picture><source media="(min-width: 50rem)" srcset="wide.png"><img alt="Media photo" srcset="narrow.png 1x"></picture>',
    [],
  ],
  [
    "picture type alternatives",
    '<picture><source type="image/unsupported" srcset="unsupported.bin"><source type="image/webp" srcset="modern.webp"><img alt="Type photo" src="fallback.png"></picture>',
    ["https://feed.test/posts/fallback.png"],
  ],
  [
    "relative fallback and responsive candidates",
    '<img alt="Fallback photo" src="../fallback.png" srcset="../large.png 2x">',
    ["https://feed.test/fallback.png"],
  ],
  [
    "picture without fallback",
    '<picture><source srcset="photo.png"><img alt="Picture photo"></picture>',
    [],
  ],
] as const;

it.each(cases)(
  "reproduces summary recovery limitations: %s",
  async (_name, source, urls) => {
    const original = source;
    expect(
      imageReferences(source, "https://feed.test/posts/article").map(
        (patch) => patch.url
      )
    ).toEqual(urls);
    const resolve = vi.fn().mockResolvedValue([]);
    const html = await prepareEntryImages(
      await markdownToHtml(source),
      "entry",
      resolve
    );
    expect(html).toContain("xaga-unavailable-image");
    expect(html).not.toMatch(/<img|<source|srcset=/);
    expect(resolve).not.toHaveBeenCalled();
    expect(source).toBe(original);
  }
);

it("renders an already-local fallback without acquiring responsive alternatives", async () => {
  const source =
    '<picture><source srcset="remote.webp"><img alt="Local photo" src="../images/entry/local.png" srcset="remote.png 2x"></picture>';
  const data = "data:image/png;base64,local";
  const resolve = vi.fn().mockResolvedValue([data]);
  const rendered = await prepareEntryImages(
    await markdownToHtml(source),
    "entry",
    resolve
  );
  expect(resolve).toHaveBeenCalledExactlyOnceWith("entry", [
    "../images/entry/local.png",
  ]);
  expect(rendered).toContain(data);
  expect(rendered).not.toContain("remote");
  expect(imageReferences(source, "https://feed.test/posts/article")).toEqual(
    []
  );
});
