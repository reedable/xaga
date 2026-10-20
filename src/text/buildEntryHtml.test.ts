// @ts-expect-error jsdom is already the Vitest browser environment dependency,
// but this project intentionally does not ship its optional declaration package.
import { JSDOM } from "jsdom";
import { afterEach, expect, it, vi } from "vitest";
import {
  buildEntryHtml,
  FIND_IN_PAGE,
  FIND_RESULT,
  HIGHLIGHT_CLICKED,
  SCROLL_TO_ANNOTATION,
  SELECTION_CLEARED,
  SELECTION_MADE,
  type HtmlReaderStyle,
  type SerializedAnnotation,
} from "./buildEntryHtml";

vi.mock("../../../lliwio-css/src/index.css?inline", () => ({ default: "" }));

const style: HtmlReaderStyle = {
  themeMode: "light",
  fontFamily: "system",
  fontSize: 16,
  lineHeight: 1.5,
};

const openDoms: JSDOM[] = [];

async function render(
  body: string,
  title: string,
  annotations: SerializedAnnotation[] = []
) {
  const dom = new JSDOM(buildEntryHtml(body, title, style, annotations), {
    pretendToBeVisual: true,
    runScripts: "dangerously",
  });
  openDoms.push(dom);
  await new Promise<void>((resolve) => {
    if (dom.window.document.readyState === "complete") resolve();
    else dom.window.addEventListener("load", () => resolve(), { once: true });
  });
  return dom;
}

afterEach(() => {
  while (openDoms.length) openDoms.pop()?.window.close();
});

it("emits the Xaga reader's bounded compact-title treatment", () => {
  const html = buildEntryHtml("<p>Body</p>", "Title", style);
  const compact = html.replace(/\s+/g, " ");

  expect(compact).toContain(
    ".xaga-reader-title { display: block; line-height: 1.25em; max-height: round(up, 20vh, 1lh); overflow-y: hidden; }"
  );
  expect(compact).toContain(
    "@media (prefers-reduced-motion: no-preference) { position: sticky; top: 0; }"
  );
  expect(compact).toContain("animation-name: header__h1--scale;");
  expect(compact).toContain("animation-timeline: scroll(nearest);");
  expect(compact).toContain(
    "border-top: 2px solid var(--lliw-semanticColor__color--link);"
  );
  expect(compact).toContain(
    "body>h1[aria-hidden] { display: none; @media (prefers-reduced-motion: no-preference) { display: block;"
  );
});

it.each([
  {
    themeMode: "light" as const,
    fontFamily: "serif" as const,
    fontSize: 18,
    lineHeight: 1.5,
    fontStack: 'Georgia,"Times New Roman",serif',
    rhythm: 27,
  },
  {
    themeMode: "dark" as const,
    fontFamily: "monospace" as const,
    fontSize: 14,
    lineHeight: 2,
    fontStack: '"Courier New",Courier,monospace',
    rhythm: 28,
  },
])(
  "preserves $themeMode reader appearance variables around the compact title",
  ({ fontStack, rhythm, ...appearance }) => {
    const html = buildEntryHtml("<p>Body</p>", "Title", appearance);

    expect(html).toContain(`color-scheme: ${appearance.themeMode};`);
    expect(html).toContain(
      `--lliw-typography__font-family: ${fontStack};`
    );
    expect(html).toContain(
      `--lliw-typography__font-size: ${appearance.fontSize}px;`
    );
    expect(html).toContain(`--lliw-typography__rhythm-unit: ${rhythm}px;`);
  }
);

it("renders the escaped canonical title in a sibling header", async () => {
  const dom = await render("<p>Body</p>", '<img src=x onerror="bad"> & title');
  const { document } = dom.window;

  expect(document.body.children[0]?.tagName).toBe("HEADER");
  expect(document.body.children[1]?.matches("h1[aria-hidden=true]")).toBe(true);
  expect(document.body.children[2]?.tagName).toBe("ARTICLE");
  expect(document.querySelector("header > h1")?.textContent).toBe(
    '<img src=x onerror="bad"> & title'
  );
  expect(document.querySelector("h1[aria-hidden]")?.textContent).toBe(
    '<img src=x onerror="bad"> & title'
  );
  expect(document.querySelector("header img")).toBeNull();
  expect(document.querySelector("h1[aria-hidden] img")).toBeNull();
});

it("restores annotation occurrences only within the article", async () => {
  const annotation: SerializedAnnotation = {
    id: "saved",
    text: ["Repeated"],
    occurrence: [1],
    color: "yellow",
    note: null,
  };
  const dom = await render("<p>Repeated and Repeated</p>", "Repeated", [
    annotation,
  ]);
  const { document } = dom.window;

  expect(document.querySelector("header mark")).toBeNull();
  expect(document.querySelector("article")?.innerHTML).toContain(
    'Repeated and <mark data-ann-id="saved"'
  );
  expect(document.querySelector("article mark")?.textContent).toBe("Repeated");
});

it("excludes aria-hidden title spacers from find results", async () => {
  const dom = await render("<p>Body</p>", "Needle");
  const postMessage = vi
    .spyOn(dom.window, "postMessage")
    .mockImplementation(() => {});
  dom.window.HTMLElement.prototype.scrollIntoView = vi.fn();

  dom.window.dispatchEvent(
    new dom.window.MessageEvent("message", {
      data: { type: FIND_IN_PAGE, query: "Needle" },
    })
  );

  expect(postMessage).toHaveBeenCalledWith(
    { type: FIND_RESULT, total: 1, current: 1 },
    "*"
  );
  expect(dom.window.document.querySelector("h1[aria-hidden] mark")).toBeNull();
});

it("keeps restored annotation clicking and scrolling functional", async () => {
  const annotation: SerializedAnnotation = {
    id: "saved",
    text: ["Target"],
    occurrence: [0],
    color: "yellow",
    note: "Note",
  };
  const dom = await render("<p>Target</p>", "Title", [annotation]);
  const mark = dom.window.document.querySelector("article mark") as HTMLElement;
  expect(mark).toBeTruthy();
  const postMessage = vi
    .spyOn(dom.window, "postMessage")
    .mockImplementation(() => {});
  const scrollIntoView = vi.fn();
  mark.scrollIntoView = scrollIntoView;

  mark.click();
  expect(postMessage).toHaveBeenCalledWith(
    expect.objectContaining({
      type: HIGHLIGHT_CLICKED,
      annotationId: "saved",
      text: "Target",
    }),
    "*"
  );

  dom.window.dispatchEvent(
    new dom.window.MessageEvent("message", {
      data: { type: SCROLL_TO_ANNOTATION, annotationId: "saved" },
    })
  );
  expect(scrollIntoView).toHaveBeenCalledWith({
    block: "center",
    behavior: "smooth",
  });
});

it.each(["header", "cross-boundary"])(
  "refuses %s annotation selections",
  async (selection) => {
    const dom = await render("<p>Body text</p>", "Header text");
    const { document } = dom.window;
    const postMessage = vi
      .spyOn(dom.window, "postMessage")
      .mockImplementation(() => {});
    const headerText = document.querySelector(
      "header h1 .xaga-reader-title"
    )?.firstChild;
    const bodyText = document.querySelector("article p")?.firstChild;
    expect(headerText).toBeTruthy();
    expect(bodyText).toBeTruthy();

    const range = document.createRange();
    range.setStart(headerText!, 0);
    if (selection === "header") range.setEnd(headerText!, 6);
    else range.setEnd(bodyText!, 4);
    dom.window.getSelection()?.removeAllRanges();
    dom.window.getSelection()?.addRange(range);
    document.dispatchEvent(
      new dom.window.MouseEvent("mouseup", { bubbles: true })
    );
    await new Promise((resolve) => dom.window.setTimeout(resolve, 20));

    expect(postMessage).toHaveBeenCalledWith({ type: SELECTION_CLEARED }, "*");
    expect(postMessage).not.toHaveBeenCalledWith(
      expect.objectContaining({ type: SELECTION_MADE }),
      "*"
    );
  }
);

it("calculates a new selection occurrence within article text only", async () => {
  const dom = await render("<p>Repeated and Repeated</p>", "Repeated");
  const { document } = dom.window;
  const postMessage = vi
    .spyOn(dom.window, "postMessage")
    .mockImplementation(() => {});
  const bodyText = document.querySelector("article p")?.firstChild;
  expect(bodyText).toBeTruthy();

  const range = document.createRange();
  range.setStart(bodyText!, 13);
  range.setEnd(bodyText!, 21);
  range.getBoundingClientRect = () => new dom.window.DOMRect(0, 0, 10, 10);
  dom.window.getSelection()?.removeAllRanges();
  dom.window.getSelection()?.addRange(range);
  document.dispatchEvent(
    new dom.window.MouseEvent("mouseup", { bubbles: true })
  );
  await new Promise((resolve) => dom.window.setTimeout(resolve, 20));

  expect(postMessage).toHaveBeenCalledWith(
    expect.objectContaining({
      type: SELECTION_MADE,
      texts: ["Repeated"],
      occurrences: [1],
    }),
    "*"
  );
});
