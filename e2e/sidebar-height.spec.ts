import {
  control,
  expect,
  openArticle,
  reader,
  startCapture,
  test,
} from "./fixtures";
import type { Page } from "@playwright/test";

const selector = (page: Page) =>
  control(page, "combobox", "sidebar-list-selector");
async function chooseList(page: Page, value: string) {
  await selector(page).click();
  await page
    .getByRole("option")
    .and(page.locator(`[data-value="${value}"]`))
    .click();
  await expect(selector(page)).toBeFocused();
}
async function layout(page: Page) {
  return page.getByTestId("sidebar-lists").evaluate((region) => ({
    available: region.getBoundingClientRect().height,
    headers: [
      ...region.querySelectorAll<HTMLElement>("[data-sidebar-section-header]"),
    ].reduce((sum, header) => sum + header.getBoundingClientRect().height, 0),
  }));
}

test("switches at the measured threshold, including equality, and tracks font size", async ({
  page,
}) => {
  await startCapture(page);
  const region = page.getByTestId("sidebar-lists");
  const initial = await layout(page);
  expect(initial.available).toBeGreaterThan(2 * initial.headers);
  await expect(selector(page)).not.toBeVisible();
  await region.evaluate((node) => {
    const total = [
      ...node.querySelectorAll<HTMLElement>("[data-sidebar-section-header]"),
    ].reduce((sum, header) => sum + header.getBoundingClientRect().height, 0);
    const rootSize = parseFloat(
      getComputedStyle(document.documentElement).fontSize
    );
    node.style.flex = "0 0 auto";
    node.style.height = `${(2 * total) / rootSize}rem`;
  });
  const equality = await layout(page);
  expect(equality.available).toBe(2 * equality.headers);
  await expect(selector(page)).not.toBeVisible();
  await region.evaluate((node) => {
    node.style.height = `calc(${node.style.height} - 0.0625rem)`;
  });
  await expect(selector(page)).toBeVisible();
  const compact = await layout(page);
  expect(compact.available).toBeLessThan(2 * compact.headers);
  expect(compact.headers).toBe(equality.headers);
  await region.evaluate((node) => {
    node.style.removeProperty("height");
    node.style.removeProperty("flex");
  });
  await page.setViewportSize({ width: 1440, height: 650 });
  await expect(selector(page)).not.toBeVisible();
  await page.addStyleTag({ content: "html { font-size: 150%; }" });
  await expect(selector(page)).toBeVisible();
  const scaled = await layout(page);
  expect(scaled.headers).toBeGreaterThan(initial.headers);
  expect(scaled.available).toBeLessThan(2 * scaled.headers);
});

for (const theme of ["light", "dark"] as const) {
  test(`compact lists preserve reader, selection and actions in ${theme}`, async ({
    page,
  }, testInfo) => {
    await startCapture(page, theme);
    await openArticle(page, "A garden for every season");
    const readingTitle = await reader(page)
      .getByRole("heading", { level: 1 })
      .textContent();
    const feedHeader = control(page, "button", "sidebar-feeds-toggle");
    await feedHeader.click();
    await feedHeader.focus();
    await page.setViewportSize({ width: 1440, height: 480 });
    await expect(selector(page)).toBeFocused();
    await expect(control(page, "list", "sidebar-feeds-list")).toBeVisible();
    await expect(
      control(page, "button", "sidebar-feeds-toggle")
    ).not.toBeVisible();
    await expect(control(page, "button", "all-items")).toBeVisible();
    await control(page, "button", "add-feed").click();
    await expect(control(page, "textbox", "feed-url")).toBeVisible();
    await page.keyboard.press("Escape");
    for (const section of ["SHORTCUTS", "TOPICS", "FEEDS", "SITES", "TAGS"]) {
      await chooseList(page, section);
      await expect(
        control(page, "list", `sidebar-${section.toLowerCase()}-list`)
      ).toBeVisible();
      await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
        readingTitle!
      );
    }
    await chooseList(page, "TOPICS");
    await control(page, "button", "add-topic").click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    const screenshotPath = testInfo.outputPath(`compact-sidebar-${theme}.png`);
    await page.screenshot({ path: screenshotPath });
    await testInfo.attach("compact-sidebar", {
      path: screenshotPath,
      contentType: "image/png",
    });
    await selector(page).focus();
    await page.setViewportSize({ width: 1440, height: 900 });
    await expect(
      control(page, "button", "sidebar-topics-toggle")
    ).toBeFocused();
    await expect(
      control(page, "button", "sidebar-topics-toggle")
    ).toHaveAttribute("aria-pressed", "true");
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      readingTitle!
    );
  });
}

test("all-collapsed state defaults to Shortcuts in compact mode without changing saved selection on resize", async ({
  page,
}) => {
  await startCapture(page);
  await control(page, "button", "sidebar-shortcuts-toggle").click();
  await page.setViewportSize({ width: 1440, height: 480 });
  await expect(control(page, "list", "sidebar-shortcuts-list")).toBeVisible();
  await page.setViewportSize({ width: 1440, height: 900 });
  for (const section of ["shortcuts", "topics", "feeds", "sites", "tags"]) {
    await expect(
      control(page, "button", `sidebar-${section}-toggle`)
    ).toHaveAttribute("aria-pressed", "false");
  }
  await page.setViewportSize({ width: 1440, height: 480 });
  await chooseList(page, "FEEDS");
  await expect(control(page, "list", "sidebar-feeds-list")).toBeVisible();
  await page.setViewportSize({ width: 1440, height: 900 });
  await expect(control(page, "button", "sidebar-feeds-toggle")).toHaveAttribute(
    "aria-pressed",
    "true"
  );
});

test("filtering reaches empty lists and clearing restores them in compact mode", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 480 });
  await startCapture(page);
  await chooseList(page, "TOPICS");
  await control(page, "button", "sidebar-sort-trigger").click();
  await control(page, "textbox", "filter-collections").fill(
    "no matching collection"
  );
  await page.keyboard.press("Escape");
  await expect(
    control(page, "list", "sidebar-topics-list").getByRole("button")
  ).toHaveCount(0);
  await chooseList(page, "FEEDS");
  await expect(
    control(page, "list", "sidebar-feeds-list").getByRole("button")
  ).toHaveCount(0);
  await control(page, "button", "sidebar-sort-trigger").click();
  await control(page, "button", "clear-collection-filter").click();
  await page.keyboard.press("Escape");
  await expect(
    control(page, "list", "sidebar-feeds-list").getByRole("button", {
      name: "Field notes",
      exact: true,
    })
  ).toBeVisible();
});

test("long lists and depth five and twenty-four remain scrollable in compact mode", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 480 });
  await page.goto("/?scenario=sidebar-height");
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await expect(selector(page)).toBeVisible();
  const shortcuts = control(page, "list", "sidebar-shortcuts-list");
  for (const title of ["Collection 39", "Collection 79"]) {
    const target = shortcuts.getByRole("button", { name: title, exact: true });
    await target.scrollIntoViewIfNeeded();
    await target.focus();
    await expect(target).toBeFocused();
    await expect(target).toBeInViewport();
  }
  await chooseList(page, "TOPICS");
  const last = control(page, "list", "sidebar-topics-list").getByRole(
    "button",
    { name: "Collection 79", exact: true }
  );
  await last.scrollIntoViewIfNeeded();
  await expect(last).toBeInViewport();
  const allItems = await control(page, "button", "all-items").boundingBox();
  expect(allItems!.y).toBeGreaterThanOrEqual(0);
});

test("compact read-only lists allow browsing and keep creation actions disabled", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 480 });
  await page.goto("/?scenario=read-only");
  await control(
    page.getByRole("dialog"),
    "button",
    "continue-read-only"
  ).click();
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await chooseList(page, "SHORTCUTS");
  await expect(control(page, "button", "add-shortcut-group")).toBeDisabled();
  await chooseList(page, "TOPICS");
  await expect(control(page, "button", "add-topic")).toBeDisabled();
  await chooseList(page, "FEEDS");
  await expect(control(page, "button", "add-feed")).toBeDisabled();
  await control(page, "list", "sidebar-feeds-list")
    .getByRole("button", { name: "Field notes", exact: true })
    .click();
  await expect(control(page, "button", "all-items")).toBeVisible();
});
