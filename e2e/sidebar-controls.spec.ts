import { expectSidebarSort, selectSidebarSort } from "./fixtures";
import { control, expect, startCapture, test } from "./fixtures";

for (const theme of ["light", "dark"] as const) {
  test(`collection popup retains and clears filtering in ${theme} theme`, async ({
    page,
  }, testInfo) => {
    await startCapture(page, theme);
    const trigger = control(page, "button", "sidebar-sort-trigger");
    const filter = control(page, "textbox", "filter-collections");
    const shortcuts = control(page, "list", "sidebar-shortcuts-list");
    const topic = shortcuts.getByRole("button", {
      name: "Everyday discoveries",
      exact: true,
    });
    await expect(topic).toBeVisible();
    await expect(filter).not.toBeVisible();
    const unfilteredName = await trigger.getAttribute("aria-label");
    await trigger.focus();
    await page.keyboard.press("Enter");
    const popup = page.getByRole("dialog");
    await expect(popup).toHaveAccessibleName(/\S/);
    const sort = control(popup, "combobox", "sidebar-sort-select");
    await expect(sort).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(filter).toBeFocused();
    await filter.fill("no matching collection");
    await page.keyboard.press("Escape");
    await expect(trigger).toBeFocused();
    await expect(topic).not.toBeVisible();
    await expect(control(page, "button", "all-items")).toBeVisible();
    await expect(trigger).toHaveAccessibleName(/\S/);
    expect(await trigger.getAttribute("aria-label")).not.toBe(unfilteredName);
    await testInfo.attach("active-collection-filter", {
      body: await trigger.screenshot(),
      contentType: "image/png",
    });
    await trigger.click();
    await expect(filter).toHaveValue("no matching collection");
    await control(popup, "button", "clear-collection-filter").click();
    await expect(filter).toBeEmpty();
    await expect(filter).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(topic).toBeVisible();
    await expect(trigger).toHaveAttribute("aria-label", unfilteredName!);
    await trigger.click();
    await filter.fill("discoveries");
    // Clicking the backdrop dismisses without changing the filter.
    await page.mouse.click(1000, 700);
    await expect(popup).not.toBeVisible();
    await trigger.click();
    await expect(filter).toHaveValue("discoveries");
  });
}

test("collection popup remains usable in a short window and scales with root text size", async ({
  page,
}, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 360 });
  await startCapture(page);
  const trigger = control(page, "button", "sidebar-sort-trigger");
  await trigger.click();
  const popup = page.getByRole("dialog");
  const initial = (await popup.boundingBox())!;
  await expect(
    control(popup, "combobox", "sidebar-sort-select")
  ).toBeInViewport();
  await expect(
    control(popup, "textbox", "filter-collections")
  ).toBeInViewport();
  await page.keyboard.press("Escape");
  await page.addStyleTag({ content: "html { font-size: 125%; }" });
  await trigger.click();
  const scaled = (await popup.boundingBox())!;
  expect(scaled.width).toBeGreaterThan(initial.width);
  expect(scaled.y).toBeGreaterThanOrEqual(0);
  expect(scaled.y + scaled.height).toBeLessThanOrEqual(
    page.viewportSize()!.height
  );
  await control(popup, "textbox", "filter-collections").fill("discoveries");
  await expect(control(popup, "textbox", "filter-collections")).toHaveValue(
    "discoveries"
  );
  await selectSidebarSort(page, "alphabetical");
  await expectSidebarSort(page, "alphabetical");
  await testInfo.attach("short-window-collection-popup", {
    body: await page.screenshot(),
    contentType: "image/png",
  });
});
