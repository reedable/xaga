import { control, expect, scrollSidebarList, test } from "./fixtures";

for (const height of [900, 480]) {
  test(`virtual collections scroll, filter, select, and resize at ${height}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height });
    await page.goto(
      `/?scenario=sidebar-height${height === 480 ? "&theme=dark" : ""}`
    );
    await expect(page.locator("html")).toHaveAttribute(
      "data-capture-ready",
      "true"
    );
    if (height === 480) {
      await control(page, "combobox", "sidebar-list-selector").click();
      await page
        .getByRole("option")
        .and(page.locator('[data-value="TOPICS"]'))
        .click();
    } else {
      await control(page, "button", "sidebar-topics-toggle").click();
    }
    const list = control(page, "list", "sidebar-topics-list");
    const first = list.getByRole("button", {
      name: "Collection 00",
      exact: true,
    });
    const last = list.getByRole("button", {
      name: "Collection 79",
      exact: true,
    });
    await expect(first).toBeVisible();
    await expect(last).toHaveCount(0);
    expect(await list.getByRole("listitem").count()).toBeLessThan(40);
    await scrollSidebarList(page, "topics", "bottom");
    await expect(last).toBeInViewport();
    await expect(first).toHaveCount(0);
    await last.click();
    await expect(last).toHaveClass(/Mui-selected/);
    await expect(last).toBeInViewport();
    await page.addStyleTag({ content: "html { font-size: 125%; }" });
    await page.setViewportSize({
      width: 1440,
      height: height === 480 ? 900 : 480,
    });
    await scrollSidebarList(page, "topics", "bottom");
    await expect(last).toBeInViewport();
    await expect(last).toHaveClass(/Mui-selected/);

    await control(page, "button", "sidebar-sort-trigger").click();
    const filter = control(page, "textbox", "filter-collections");
    await filter.fill("Collection 00");
    await page.keyboard.press("Escape");
    await expect(first).toBeInViewport();
    await expect(last).toHaveCount(0);
    await control(page, "button", "sidebar-sort-trigger").click();
    await filter.fill("no matching collection");
    await page.keyboard.press("Escape");
    await expect(list.getByRole("listitem")).toHaveCount(0);
    await expect(list.locator("[data-virtuoso-scroller]")).toHaveCount(0);
    await control(page, "button", "sidebar-sort-trigger").click();
    await control(page, "button", "clear-collection-filter").click();
    await page.keyboard.press("Escape");
    await expect(first).toBeVisible();
    await scrollSidebarList(page, "topics", "bottom");
    await expect(last).toBeInViewport();
    await expect(last).toHaveClass(/Mui-selected/);
  });
}

test("Tab and Shift+Tab traverse virtual rows and leave the list at its boundaries", async ({
  page,
}) => {
  test.setTimeout(60_000);
  await page.setViewportSize({ width: 1440, height: 480 });
  await page.goto("/?scenario=sidebar-height");
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  const selector = control(page, "combobox", "sidebar-list-selector");
  await selector.click();
  await page
    .getByRole("option")
    .and(page.locator('[data-value="TOPICS"]'))
    .click();
  const list = control(page, "list", "sidebar-topics-list");
  await control(page, "button", "sidebar-sort-trigger").click();
  await control(page, "textbox", "filter-collections").fill("Collection");
  await page.keyboard.press("Escape");
  await list
    .getByRole("button", { name: "Collection 00", exact: true })
    .focus();
  for (let index = 1; index < 80; index++) {
    await page.keyboard.press("Tab");
    const row = list.getByRole("button", {
      name: `Collection ${String(index).padStart(2, "0")}`,
      exact: true,
    });
    await expect(row).toBeFocused();
    await expect(row).toBeInViewport();
  }
  for (let index = 78; index >= 0; index--) {
    await page.keyboard.press("Shift+Tab");
    await expect(
      list.getByRole("button", {
        name: `Collection ${String(index).padStart(2, "0")}`,
        exact: true,
      })
    ).toBeFocused();
  }
  await page.keyboard.press("Shift+Tab");
  await expect(control(page, "button", "add-topic")).toBeFocused();
  await scrollSidebarList(page, "topics", "bottom");
  await list
    .getByRole("button", { name: "Collection 79", exact: true })
    .focus();
  await page.keyboard.press("Tab");
  await expect(
    list.getByRole("button", { name: "Collection 79", exact: true })
  ).not.toBeFocused();
});

test("tag rows retain Tab order between selection and actions", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await control(page, "button", "sidebar-tags-toggle").click();
  const list = control(page, "list", "sidebar-tags-list");
  const choices = list.getByRole("checkbox");
  const actions = list.getByRole("button");
  await expect(choices).toHaveCount(2);
  await expect(actions).toHaveCount(2);
  await choices.first().focus();
  await page.keyboard.press("Tab");
  await expect(actions.first()).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(choices.last()).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(actions.first()).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(choices.first()).toBeFocused();
});
