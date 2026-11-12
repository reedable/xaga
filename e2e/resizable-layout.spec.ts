import {
  test,
  expect,
  startCapture,
  openArticle,
  reader,
  control,
  setSidebarVisible,
} from "./fixtures";
import type { Page } from "@playwright/test";

const garden = "A garden for every season";
const sidebar = (page: Page) => page.getByTestId("sidebar-column-resize");
const list = (page: Page) => page.getByTestId("entity-list-column-resize");
const query = (page: Page) => control(page, "textbox", "search-query");

async function width(page: Page, id: string) {
  return page
    .locator(`#${id}`)
    .evaluate(
      (el) =>
        el.getBoundingClientRect().width /
        parseFloat(getComputedStyle(document.documentElement).fontSize)
    );
}

async function drag(page: Page, id: string, delta: number) {
  const box = (await page.getByTestId(id).boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + delta, box.y + box.height / 2, {
    steps: 5,
  });
  await page.mouse.up();
}

test("columns resize independently with pointer and keyboard within their limits", async ({
  page,
}) => {
  await startCapture(page);
  await drag(page, "sidebar-column-resize", 1000);
  expect(await width(page, "sidebar-column")).toBe(22.5);
  expect(await width(page, "entity-list-column")).toBe(20);
  await drag(page, "entity-list-column-resize", 1000);
  expect(await width(page, "entity-list-column")).toBe(30);
  expect(await width(page, "sidebar-column")).toBe(22.5);
  for (const [handle, min, max] of [
    [sidebar(page), 15, 22.5],
    [list(page), 20, 30],
  ] as const) {
    await expect(handle).toHaveAccessibleName(/\S/);
    await handle.focus();
    await page.keyboard.press("Home");
    await expect(handle).toHaveAttribute("aria-valuenow", String(min));
    await page.keyboard.press("ArrowLeft");
    await expect(handle).toHaveAttribute("aria-valuenow", String(min));
    await page.keyboard.press("ArrowRight");
    await expect(handle).toHaveAttribute("aria-valuenow", String(min + 0.0625));
    await page.keyboard.press("End");
    await page.keyboard.press("ArrowRight");
    await expect(handle).toHaveAttribute("aria-valuenow", String(max));
    expect(
      await handle.evaluate((el) => getComputedStyle(el).outlineStyle)
    ).not.toBe("none");
  }
});

test("Sidebar follows its preference across window resizing and restores its width", async ({
  page,
}) => {
  await startCapture(page);
  await sidebar(page).focus();
  await page.keyboard.press("End");
  for (const width of [979, 980, 981, 1299, 1300, 1301]) {
    await page.setViewportSize({ width, height: 800 });
    await expect(sidebar(page)).toHaveCount(1);
    expect(
      await page
        .locator("#sidebar-column")
        .evaluate(
          (el) =>
            el.getBoundingClientRect().width /
            parseFloat(getComputedStyle(document.documentElement).fontSize)
        )
    ).toBe(22.5);
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  await openArticle(page, garden);
  await control(page, "button", "search").click();
  for (const width of [1299, 1300, 1301]) {
    await page.setViewportSize({ width, height: 800 });
    await expect(sidebar(page)).toHaveCount(1);
  }
  await page.keyboard.press("Escape");
  await setSidebarVisible(page, false);
  await expect(control(page, "combobox", "collection-selector")).toBeVisible();
  await setSidebarVisible(page, true);
  expect(await width(page, "sidebar-column")).toBe(22.5);
});

for (const [root, wide] of [
  [16, false],
  [16, true],
  [17, true],
  [20, false],
  [20, true],
] as const) {
  test(`Search uses available space at exact boundaries, root ${root}, wider columns ${wide}`, async ({
    page,
  }) => {
    await startCapture(page);
    await openArticle(page, garden);
    if (wide) {
      await sidebar(page).focus();
      await page.keyboard.press("End");
      await list(page).focus();
      await page.keyboard.press("End");
    }
    await page.evaluate((size) => {
      document.documentElement.style.fontSize = `${size}px`;
    }, root);
    const columns = wide ? 52.5 : 35;
    const boundary = (columns + 45) * root + 1;
    const drawerInput = page
      .getByRole("dialog", { includeHidden: true })
      .filter({ has: page.locator('input[name="search-query"]') })
      .locator('input[name="search-query"]');
    const drawerDraft = await drawerInput.inputValue();
    await page.setViewportSize({ width: Math.ceil(boundary), height: 800 });
    await control(page, "button", "search").click();
    await query(page).fill("garden");
    await expect(page).toHaveURL(/search=garden/);
    const result = page
      .getByTestId("search-inline")
      .getByRole("button", { name: new RegExp(`^${garden}`) });
    await expect(result).toBeVisible();
    for (const delta of [-1, 0, 1, -1, 1]) {
      await page.setViewportSize({
        width: Math.ceil(boundary) + delta,
        height: 800,
      });
      const modal = page.getByRole("dialog");
      if (delta < 0) {
        await expect(modal).toBeVisible();
        await expect(query(page)).toBeFocused();
      } else {
        await expect(modal).not.toBeVisible();
        await expect(page.getByTestId("search-inline")).toBeVisible();
      }
      await expect(page).toHaveURL(/search=garden/);
      await expect(query(page)).toHaveValue(delta < 0 ? drawerDraft : "garden");
      const searchScope = delta < 0 ? modal : page.getByTestId("search-inline");
      await expect(
        searchScope.getByRole("button", { name: new RegExp(`^${garden}`) })
      ).toBeVisible();
      await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
        garden
      );
      const geometry = await page.getByTestId("entry-reader").evaluate((el) => {
        const box = el.getBoundingClientRect();
        const pane = document
          .querySelector('[data-testid="reading-space"]')!
          .getBoundingClientRect();
        return {
          left: box.left,
          paneLeft: pane.left,
          scroll: el.parentElement!.scrollLeft,
        };
      });
      expect(geometry.left).toBeGreaterThanOrEqual(geometry.paneLeft);
      expect(geometry.scroll).toBe(0);
    }
    await page.setViewportSize({ width: Math.ceil(boundary) - 1, height: 800 });
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.press("Tab");
    expect(
      await page
        .getByRole("dialog")
        .evaluate((el) => el.contains(document.activeElement))
    ).toBe(true);
    await page.keyboard.press("Escape");
    await expect(query(page)).toHaveCount(0);
    await expect(control(page, "button", "search")).toBeEnabled();
    await control(page, "button", "search").focus();
    await page.keyboard.press("Enter");
    await expect(query(page)).toBeFocused();
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      garden
    );
  });
}

test("live column dragging changes Search presentation without losing results or collection", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1550, height: 900 });
  await startCapture(page);
  await openArticle(page, garden);
  await control(page, "button", "search").click();
  await query(page).fill("garden");
  const inline = page.getByTestId("search-inline");
  await expect(
    inline.getByRole("button", { name: new RegExp(`^${garden}`) })
  ).toBeVisible();
  await drag(page, "entity-list-column-resize", 160);
  await drag(page, "sidebar-column-resize", 120);
  const drawer = page.getByRole("dialog");
  await expect(drawer).toBeVisible();
  await expect(
    drawer.getByRole("button", { name: new RegExp(`^${garden}`) })
  ).toBeVisible();
  await expect(page).toHaveURL(/search=garden/);
  await expect(query(page)).toBeFocused();
  // A backdrop click closes Search without selecting a different collection/article.
  await page.mouse.click(800, 700);
  await expect(drawer).not.toBeVisible();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  await expect(control(page, "button", "filter-unread")).toHaveAttribute(
    "aria-pressed",
    "true"
  );
  expect(await width(page, "sidebar-column")).toBe(22.5);
  expect(await width(page, "entity-list-column")).toBe(30);
});

test("selecting a drawer result and dismissing Search preserves the selected article", async ({
  page,
}) => {
  await page.setViewportSize({ width: 900, height: 800 });
  await startCapture(page);
  await openArticle(page, garden);
  await control(page, "button", "search").click();
  await query(page).fill("workshop");
  const drawer = page.getByRole("dialog");
  const title = "Making room for a workshop";
  await drawer.getByRole("button", { name: new RegExp(`^${title}`) }).click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    title
  );
  await page.keyboard.press("Escape");
  await expect(drawer).not.toBeVisible();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    title
  );
  await expect(control(page, "button", "filter-unread")).toHaveAttribute(
    "aria-pressed",
    "true"
  );
});

test("a pointer burst applies its last width once per frame and cancellation retains it", async ({
  page,
}) => {
  await startCapture(page);
  const handle = sidebar(page);
  const box = (await handle.boundingBox())!;
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  const updates = await handle.evaluate(async (element, start) => {
    const column = element.parentElement!;
    let changes = 0;
    const observer = new MutationObserver((records) => {
      changes += records.length;
    });
    observer.observe(column, { attributes: true, attributeFilter: ["style"] });
    for (let offset = 1; offset <= 64; offset++)
      element.dispatchEvent(
        new PointerEvent("pointermove", {
          bubbles: true,
          clientX: start + offset,
          pointerId: 1,
        })
      );
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => resolve())
    );
    observer.disconnect();
    return changes;
  }, x);
  expect(updates).toBe(1);
  expect(await width(page, "sidebar-column")).toBe(19);
  await expect(handle).toHaveAttribute("aria-valuenow", "19");
  await handle.dispatchEvent("pointercancel");
  await page.mouse.up();
  await setSidebarVisible(page, false);
  await expect(handle).toHaveCount(0);
  await setSidebarVisible(page, true);
  await expect(handle).toHaveAttribute("aria-valuenow", "19");
  expect(await width(page, "sidebar-column")).toBe(19);
});

test("Search slides over the fixed-width Inspector while the Article pane remains stable", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await startCapture(page);
  await openArticle(page, garden);
  const before = await page.getByTestId("entry-reader").boundingBox();
  await page.evaluate(() => {
    const pane = document.querySelector('[data-testid="entry-reader"]')!;
    const inspector = document.querySelector("aside")!;
    const content = inspector.firstElementChild!;
    const frames: {
      articleX: number;
      articleWidth: number;
      inspectorWidth: number;
      contentWidth: number;
      mounted: boolean;
      searchWidth: number;
    }[] = [];
    (window as unknown as { searchFrames: typeof frames }).searchFrames =
      frames;
    const recording = { complete: false };
    (
      window as unknown as { searchRecording: typeof recording }
    ).searchRecording = recording;
    const record = () => {
      const start = performance.now();
      const sample = () => {
        const article = pane.getBoundingClientRect();
        const search = document
          .querySelector('[data-testid="search-inline"]')
          ?.closest(".MuiCollapse-root");
        frames.push({
          articleX: article.x,
          articleWidth: article.width,
          inspectorWidth: inspector.getBoundingClientRect().width,
          contentWidth: content.getBoundingClientRect().width,
          mounted: inspector.isConnected && content.isConnected,
          searchWidth: search?.getBoundingClientRect().width ?? 0,
        });
        if (performance.now() - start < 650) requestAnimationFrame(sample);
        else recording.complete = true;
      };
      sample();
    };
    document
      .querySelector('[data-testid="search"]')!
      .addEventListener("click", record, { once: true, capture: true });
  });
  await control(page, "button", "search").click();
  await expect(query(page)).toBeFocused();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as unknown as { searchRecording: { complete: boolean } })
            .searchRecording.complete
      )
    )
    .toBe(true);
  const frames = await page.evaluate(
    () =>
      (
        window as unknown as {
          searchFrames: {
            articleX: number;
            articleWidth: number;
            inspectorWidth: number;
            contentWidth: number;
            mounted: boolean;
            searchWidth: number;
          }[];
        }
      ).searchFrames
  );
  expect(
    frames.some((frame) => frame.searchWidth > 0 && frame.searchWidth < 288)
  ).toBe(true);
  for (const frame of frames) {
    expect(frame.mounted).toBe(true);
    expect(frame.articleX).toBeCloseTo(before!.x, 1);
    expect(frame.articleWidth).toBeCloseTo(before!.width, 1);
    expect(frame.inspectorWidth).toBe(288);
    expect(frame.contentWidth).toBe(288);
  }
  await page.keyboard.press("Escape");
  await expect(control(page, "button", "search")).toBeEnabled();
  await expect
    .poll(
      async () => (await page.getByTestId("entry-reader").boundingBox())!.width
    )
    .toBeCloseTo(before!.width, 1);
  await control(page, "button", "search").focus();
  await page.keyboard.press("Enter");
  await expect(query(page)).toBeFocused();
});

for (const theme of ["light", "dark"] as const) {
  test(`drawer Search matches inline spacing, filters, and result dividers in ${theme}`, async ({
    page,
  }) => {
    await startCapture(page, theme);
    await openArticle(page, garden);
    await control(page, "button", "search").click();
    await query(page).fill("garden");
    const inline = page.getByTestId("search-inline");
    await expect(
      inline.getByRole("button", { name: new RegExp(`^${garden}`) })
    ).toBeVisible();
    const inlineStyle = await inline.evaluate((element) => {
      const content = element.firstElementChild!;
      const filter = content.querySelector('button[value="entries"]')!;
      const result = content.querySelector(".MuiList-root > div")!;
      return {
        padding: getComputedStyle(content).paddingTop,
        filterBorder: getComputedStyle(filter).border,
        filterPadding: getComputedStyle(filter).padding,
        resultBorder: getComputedStyle(result).borderBottom,
      };
    });
    await page.setViewportSize({ width: 1000, height: 900 });
    const drawer = page.getByRole("dialog");
    await expect(drawer).toBeVisible();
    await expect(
      drawer.getByRole("button", { name: new RegExp(`^${garden}`) })
    ).toBeVisible();
    const drawerStyle = await drawer.evaluate((element) => {
      const content = element.querySelector(".MuiList-root")!.parentElement!;
      const filter = content.querySelector('button[value="entries"]')!;
      const result = content.querySelector(".MuiList-root > div")!;
      return {
        padding: getComputedStyle(content).paddingTop,
        filterBorder: getComputedStyle(filter).border,
        filterPadding: getComputedStyle(filter).padding,
        resultBorder: getComputedStyle(result).borderBottom,
      };
    });
    expect(drawerStyle).toEqual(inlineStyle);
    const background = await drawer.evaluate((element) => ({
      paper: getComputedStyle(element).backgroundColor,
      body: getComputedStyle(document.body).backgroundColor,
      overlay: getComputedStyle(element).backgroundImage,
    }));
    expect(background.paper).toBe(background.body);
    expect(background.overlay).toBe("none");
  });
}
