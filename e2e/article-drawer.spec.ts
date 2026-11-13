import type { Page } from "@playwright/test";
import {
  test,
  expect,
  startCapture,
  openArticle,
  articleButton,
  reader,
  control,
  shareAction,
} from "./fixtures";

const garden = "A garden for every season";
const water = "Following the water";
const pane = (page: Page) => page.getByTestId("article-pane");
const close = (page: Page) => control(page, "button", "close-article");
const heading = (page: Page) => reader(page).getByRole("heading", { level: 1 });

async function collection(page: Page) {
  await control(page, "button", "sidebar-feeds-toggle").click();
  await control(page, "list", "sidebar-feeds-list")
    .getByRole("button", { name: "Field notes", exact: true })
    .click();
  await control(page, "button", "filter-all").click();
}

async function history(page: Page) {
  return page.evaluate(() => structuredClone(window.history.state));
}

test("narrow launch, collection browsing, dismissal, reselection and Back preserve article history", async ({
  page,
}) => {
  await page.setViewportSize({ width: 800, height: 800 });
  await startCapture(page);
  await collection(page);
  await openArticle(page, garden);
  await expect(pane(page)).toHaveAttribute("aria-modal", "true");
  const first = await history(page);
  await close(page).click();
  await expect(pane(page)).not.toBeVisible();
  await expect(articleButton(page, garden)).toBeFocused();
  await expect(control(page, "button", "back")).toHaveCount(0);
  expect(await history(page)).toEqual(first);
  await openArticle(page, garden);
  expect(await history(page)).toEqual(first);
  await close(page).click();
  await openArticle(page, water);
  await control(page, "button", "back").click();
  await expect(heading(page)).toHaveText(garden);
  await expect(pane(page)).toBeVisible();
  await control(page, "button", "forward").click();
  await expect(heading(page)).toHaveText(water);
});

test("wide navigation, narrow history and widening preserve the selected route", async ({
  page,
}) => {
  await startCapture(page);
  await collection(page);
  await openArticle(page, garden);
  await openArticle(page, water);
  const selected = await history(page);
  await page.setViewportSize({ width: 800, height: 800 });
  await expect(pane(page)).toHaveAttribute("data-presentation", "drawer");
  await expect(pane(page)).toBeVisible();
  expect(await history(page)).toEqual(selected);
  await control(page, "button", "back").click();
  await expect(heading(page)).toHaveText(garden);
  await control(page, "button", "forward").click();
  await expect(heading(page)).toHaveText(water);
  const restored = await history(page);
  expect(restored.idx).toBe(selected.idx);
  expect(restored.usr).toEqual(selected.usr);
  await page.setViewportSize({ width: 1440, height: 900 });
  await expect(pane(page)).toHaveAttribute("data-presentation", "inline");
  expect(await history(page)).toEqual(restored);
  await control(page, "button", "back").click();
  await expect(heading(page)).toHaveText(garden);
});

for (const rootSize of [16, 20]) {
  test(`threshold equality, adjacent widths, drawer bounds and DOM identity at root ${rootSize}`, async ({
    page,
  }) => {
    await startCapture(page);
    await openArticle(page, garden);
    // Retain real node references and their whole ancestor chain across changes.
    const identity = await page
      .getByTestId("entry-reader")
      .evaluateHandle((frame) => {
        const chain: Node[] = [];
        for (let node: Node | null = frame; node; node = node.parentNode)
          chain.push(node);
        return {
          frame,
          chain,
          window: (frame as HTMLIFrameElement).contentWindow,
        };
      });
    const selected = await history(page);
    await page.evaluate((size) => {
      document.documentElement.style.fontSize = `${size}px`;
    }, rootSize);
    const boundary = 55 * rootSize + 1;
    for (const delta of [-1, 0, 1, -1]) {
      await page.setViewportSize({ width: boundary + delta, height: 800 });
      await expect(pane(page)).toHaveAttribute(
        "data-presentation",
        delta < 0 ? "drawer" : "inline"
      );
      await expect(pane(page)).toBeVisible();
      expect(await history(page)).toEqual(selected);
      await expect(heading(page)).toHaveText(garden);
      expect(
        await identity.evaluate(({ frame, chain, window: originalWindow }) => {
          let current: Node | null = frame;
          return (
            (frame as HTMLIFrameElement).contentWindow === originalWindow &&
            chain.every((node) => {
              const same = node === current;
              current = current?.parentNode ?? null;
              return same;
            })
          );
        })
      ).toBe(true);
    }
    const bounds = await pane(page).evaluate((element) => ({
      width:
        element.getBoundingClientRect().width /
        parseFloat(getComputedStyle(document.documentElement).fontSize),
      right: element.getBoundingClientRect().right,
      viewport: innerWidth,
    }));
    expect(bounds.width).toBeGreaterThanOrEqual(20);
    expect(bounds.width).toBeLessThanOrEqual(46.5);
    expect(bounds.right).toBe(bounds.viewport);
    await close(page).click();
    await expect(pane(page)).not.toBeVisible();
    await openArticle(page, garden);
    expect(
      await identity.evaluate(
        ({ frame, chain }) => frame.isConnected && frame.parentNode === chain[1]
      )
    ).toBe(true);
    await page.setViewportSize({ width: 20 * rootSize, height: 16 * rootSize });
    await expect(close(page)).toBeInViewport();
    for (const id of [
      "back",
      "forward",
      "original-site",
      "refresh",
      "share",
      "bookmark-toggle",
      "article-details",
      "search",
    ]) {
      await expect(control(pane(page), "button", id)).toBeInViewport();
      await expect(control(pane(page), "button", id)).toHaveAccessibleName(
        /\S/
      );
    }
  });
}

test("column resizing changes presentation using the independent inline budget", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1000, height: 800 });
  await startCapture(page);
  await openArticle(page, garden);
  const list = page.getByTestId("entity-list-column-resize");
  await list.focus();
  await page.keyboard.press("End");
  await expect(pane(page)).toHaveAttribute("data-presentation", "drawer");
  await expect(pane(page)).toBeVisible();
  await close(page).click();
  await list.focus();
  await page.keyboard.press("Home");
  await expect(pane(page)).toHaveAttribute("data-presentation", "inline");
  await expect(heading(page)).toHaveText(garden);
});

test("address editing, iframe Find, Search and details dismiss before the article", async ({
  page,
}) => {
  await page.setViewportSize({ width: 640, height: 800 });
  await startCapture(page);
  await openArticle(page, garden);
  const selected = await history(page);
  const address = pane(page).locator("input").first();
  await address.fill("unfinished.invalid");
  await page.keyboard.press("Escape");
  await expect(pane(page)).toBeVisible();
  expect(await history(page)).toEqual(selected);
  await reader(page).locator("article").click();
  await page.keyboard.press("Control+f");
  const find = pane(page).locator('input[name="article-find"]');
  await expect(find).toBeFocused();
  await find.fill("garden");
  await reader(page).locator("article").click();
  await page.keyboard.press("Escape");
  await expect(find).toHaveCount(0);
  await expect(pane(page)).toBeVisible();
  await control(pane(page), "button", "share").click();
  await expect(control(page, "menuitem", "open-browser")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(control(page, "menuitem", "open-browser")).toHaveCount(0);
  await expect(pane(page)).toBeVisible();
  await control(pane(page), "button", "bookmark-toggle").click();
  await expect(page.getByTestId("bookmark-palette")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("bookmark-palette")).toHaveCount(0);
  await expect(pane(page)).toBeVisible();
  await control(page, "button", "article-details").click();
  await expect(page.locator("[data-inspector-drawer]")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator("[data-inspector-drawer]")).not.toBeVisible();
  await expect(pane(page)).toBeVisible();
  await control(page, "button", "search").click();
  await expect(control(page, "textbox", "search-query")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(control(page, "textbox", "search-query")).toHaveCount(0);
  await expect(pane(page)).toBeVisible();
  await reader(page).locator("article").click();
  await page.keyboard.press("Escape");
  await expect(pane(page)).not.toBeVisible();
  await expect(articleButton(page, garden)).toBeFocused();
});

test("backdrop dismissal and virtualized focus return retain the reader", async ({
  page,
}) => {
  await startCapture(page);
  await control(page, "button", "filter-all").click();
  await openArticle(page, garden);
  await page.addStyleTag({
    content: '[data-testid="entity-list-scroller"] { max-height: 4rem; }',
  });
  const scroller = page.getByTestId("entity-list-scroller");
  await scroller.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await page.setViewportSize({ width: 800, height: 800 });
  await expect(pane(page)).toBeVisible();
  await page
    .getByTestId("article-backdrop")
    .click({ position: { x: 10, y: 200 } });
  await expect(articleButton(page, garden)).toBeFocused();
  await openArticle(page, garden);
  // The selected article can be absent from the current list after a text filter.
  await close(page).click();
  await page.getByRole("textbox").fill("no-matching-fixture");
  await page.setViewportSize({ width: 1440, height: 900 });
  await expect(pane(page)).toHaveAttribute("data-presentation", "inline");
  await page.setViewportSize({ width: 800, height: 800 });
  await close(page).click();
  await expect(page.getByTestId("entity-list-header")).toBeFocused();
});

test("annotation and article-shortcut activation reopen content in the current presentation", async ({
  page,
}) => {
  await startCapture(page);
  await openArticle(page, garden);
  await shareAction(page, "pin-toggle");
  await control(page, "button", "filter-all").click();
  const annotation = page.getByRole("button", {
    name: /^Begin with one question/,
  });
  await page.setViewportSize({ width: 800, height: 800 });
  await close(page).click();
  await annotation.click();
  await expect(pane(page)).toBeVisible();
  await expect(heading(page)).toHaveText("Making room for a workshop");
  await close(page).click();
  await page.getByRole("button", { name: garden, exact: true }).click();
  await expect(pane(page)).toBeVisible();
  await expect(heading(page)).toHaveText(garden);
});

test("URL actions display their destination inside the article drawer", async ({
  page,
}) => {
  await startCapture(page);
  await control(page, "button", "filter-all").click();
  await openArticle(page, water);
  const destination = await pane(page).locator("input").first().inputValue();
  await openArticle(page, garden);
  await page.setViewportSize({ width: 800, height: 800 });
  const address = pane(page).locator("input").first();
  await address.fill(destination);
  await address.press("Enter");
  await expect(heading(page)).toHaveText(water);
  await expect(pane(page)).toBeVisible();
});

test("reader document, scroll and Find state survive presentation changes and dismissal", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1100, height: 500 });
  await startCapture(page);
  await openArticle(page, garden);
  await reader(page).locator("article p").last().scrollIntoViewIfNeeded();
  const documentIdentity = await reader(page)
    .locator("html")
    .evaluateHandle((element) => ({
      element,
      document: element.ownerDocument,
    }));
  const scroll = await reader(page)
    .locator("html")
    .evaluate((element) => element.ownerDocument.scrollingElement!.scrollTop);
  expect(scroll).toBeGreaterThan(0);
  await reader(page).locator("article p").last().click();
  await page.keyboard.press("Control+f");
  const find = pane(page).locator('input[name="article-find"]');
  await find.fill("unmatched reader query");
  await page.setViewportSize({ width: 800, height: 500 });
  await expect(pane(page)).toHaveAttribute("data-presentation", "drawer");
  await expect(find).toHaveValue("unmatched reader query");
  const narrowScroll = await reader(page)
    .locator("html")
    .evaluate((element) => element.ownerDocument.scrollingElement!.scrollTop);
  expect(narrowScroll).toBeGreaterThan(0);
  await close(page).click();
  await openArticle(page, garden);
  expect(
    await reader(page)
      .locator("html")
      .evaluate((element) => element.ownerDocument.scrollingElement!.scrollTop)
  ).toBe(narrowScroll);
  await expect(find).toHaveValue("unmatched reader query");
  await page.setViewportSize({ width: 1100, height: 500 });
  await expect(pane(page)).toHaveAttribute("data-presentation", "inline");
  await expect(find).toHaveValue("unmatched reader query");
  expect(
    await documentIdentity.evaluate(
      ({ element, document: originalDocument }) =>
        element.isConnected && element === originalDocument.documentElement
    )
  ).toBe(true);
});
