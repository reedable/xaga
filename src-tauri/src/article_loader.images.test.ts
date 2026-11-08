import { readFileSync } from "node:fs";
import { JSDOM } from "jsdom";
import { afterEach, expect, it, vi } from "vitest";
import { processEntry } from "../../src/text/entryWorker";
import { markdownToHtml } from "../../src/text/markdownPipeline";
import { prepareEntryImages } from "../../src/text/prepareEntryImages";

const script = readFileSync("src-tauri/src/article_loader.js", "utf8");
const data =
  "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
const prose = `<p>${"Ordinary article prose provides enough meaningful text for extraction. ".repeat(30)}</p>`;
let dom: JSDOM | undefined;
function loader(body: string, enabled: boolean, keepOpen = false) {
  dom = new JSDOM(
    `<!doctype html><html><head><title>Article</title></head><body><article>${prose}${body}${prose}</article></body></html>`,
    {
      url: "https://site.test/articles/story",
      runScripts: "outside-only",
    }
  );
  const invoke = vi.fn().mockResolvedValue(undefined);
  Object.assign(dom.window, {
    __LOAD_REMOTE_IMAGE__: enabled,
    __LOADER_KEEP_OPEN__: keepOpen,
    __TAURI_INTERNALS__: { invoke },
  });
  dom.window.eval(script);
  // The test drives delivery explicitly instead of waiting for page settling.
  dom.window.eval(
    "__xagaLoadHandled = true; __xagaFaviconUrl = function() { return null; }"
  );
  return {
    window: dom.window,
    img: dom.window.document.querySelector("img")!,
    invoke,
  };
}
afterEach(() => {
  dom?.window.close();
  vi.useRealTimers();
});

it.each([
  [
    "srcset only",
    '<img srcset="/selected.png 1x" alt="photo" title="caption">',
  ],
  [
    "picture with fallback",
    '<picture><source srcset="/selected.png"><img src="/fallback.png" alt="photo" title="caption"></picture>',
  ],
  [
    "picture without fallback",
    '<picture><source srcset="/selected.png"><img alt="photo" title="caption"></picture>',
  ],
])(
  "retains %s through disabled/failed capture and Markdown rendering",
  async (_name, body) => {
    for (const enabled of [false, true]) {
      const { window, img } = loader(body, enabled);
      const selected = "https://site.test/selected.png";
      Object.defineProperty(img, "currentSrc", {
        value: selected,
        configurable: true,
      });
      const fetchImage = vi.fn().mockResolvedValue(null);
      Object.assign(window, { __xagaFetchAsDataUrl: fetchImage });
      await window.eval("__xagaFetchImages()");
      const original = window.document.documentElement.outerHTML;
      const html = window.eval("__xagaSnapshotHtml()") as string;
      expect(window.document.documentElement.outerHTML).toBe(original);
      expect(html).toContain(`data-xaga-selected-src="${selected}"`);
      const { markdown, imageData } = await processEntry(
        window.location.href,
        html
      );
      expect(markdown).toContain(`![photo](${selected} "caption")`);
      expect(markdown).not.toContain("fallback.png");
      expect(imageData).toEqual({});
      expect(fetchImage).toHaveBeenCalledTimes(enabled ? 1 : 0);
      // Retaining a URL does not grant the reading pane remote-image access.
      const resolve = vi.fn();
      const rendered = await prepareEntryImages(
        await markdownToHtml(markdown),
        "entry",
        resolve
      );
      expect(rendered).toContain("xaga-unavailable-image");
      expect(rendered).toContain("photo");
      expect(rendered).toContain("caption");
      expect(rendered).not.toContain("<img");
      expect(resolve).not.toHaveBeenCalled();
      window.close();
    }
  }
);

it.each([
  [
    '<img src="../fallback.png" srcset="/candidate.png 1x" alt="photo">',
    "https://site.test/fallback.png",
  ],
  [
    '<picture><source srcset="/candidate.png"><img alt="photo"></picture>',
    undefined,
  ],
])(
  "uses existing src, without selecting a candidate, when currentSrc is unavailable: %s",
  async (body, expected) => {
    const { window } = loader(body, false);
    const { markdown, imageData } = await processEntry(
      window.location.href,
      window.eval("__xagaSnapshotHtml()")
    );
    if (expected) expect(markdown).toContain(expected);
    expect(markdown).not.toContain("candidate.png");
    expect(imageData).toEqual({});
  }
);

it("keeps successful capture paired with its fetched URL even if currentSrc changes", async () => {
  const { window, img } = loader(
    '<picture><source srcset="/selected.png"><img src="/fallback.png" alt="photo" title="caption"></picture>',
    true
  );
  const selected = "https://site.test/selected.png";
  Object.defineProperty(img, "currentSrc", {
    value: selected,
    configurable: true,
  });
  Object.assign(window, {
    __xagaFetchAsDataUrl: vi.fn().mockResolvedValue(data),
  });
  await window.eval("__xagaFetchImages()");
  Object.defineProperty(img, "currentSrc", {
    value: "https://site.test/later.png",
  });
  const { markdown, imageData } = await processEntry(
    window.location.href,
    window.eval("__xagaSnapshotHtml()")
  );
  expect(markdown).toContain(`![photo](${selected} "caption")`);
  expect(markdown).not.toContain("later.png");
  expect(imageData).toEqual({ [selected]: data });
});

it("uses the snapshot for deadline and visible-loader delivery", async () => {
  const { window, img, invoke } = loader(
    '<img srcset="/selected.png 1x">',
    false
  );
  Object.defineProperty(img, "currentSrc", {
    value: "https://site.test/selected.png",
  });
  window.eval("window.__xagaCaptureOnTimeout()");
  expect(
    invoke.mock.calls.find(([name]) => name === "receive_html")?.[1].html
  ).toContain('data-xaga-selected-src="https://site.test/selected.png"');
  window.eval("window.__captureAndClose()");
  await vi.waitFor(() =>
    expect(invoke).toHaveBeenCalledWith(
      "capture_and_close_loader",
      expect.anything()
    )
  );
  expect(
    invoke.mock.calls.find(([name]) => name === "capture_and_close_loader")?.[1]
      .html
  ).toContain('data-xaga-selected-src="https://site.test/selected.png"');
});

it("preserves the selected URL for normal and SPA delivery", async () => {
  const { window, img, invoke } = loader(
    '<img srcset="/selected.png 1x" alt="photo">',
    false,
    true
  );
  Object.defineProperty(img, "currentSrc", {
    value: "https://site.test/selected.png",
  });
  window.eval("__xagaLoadHandled = false");
  window.dispatchEvent(new window.Event("load"));
  await vi.waitFor(() =>
    expect(invoke).toHaveBeenCalledWith(
      "receive_html",
      expect.objectContaining({
        html: expect.stringContaining(
          'data-xaga-selected-src="https://site.test/selected.png"'
        ),
      })
    )
  );
  invoke.mockClear();
  window.history.pushState({}, "", "/next-article");
  await vi.waitFor(() =>
    expect(invoke).toHaveBeenCalledWith(
      "loader_page_loaded",
      expect.objectContaining({
        html: expect.stringContaining(
          'data-xaga-selected-src="https://site.test/selected.png"'
        ),
      })
    )
  );
});
