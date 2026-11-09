import { test, expect, startCapture, openArticle } from "./fixtures";
const additionalFeedUrl = "https://workshop.example/feed.xml";

test("one keyboard-accessible sidebar sort choice applies across sections and survives restart", async ({
  page,
}, testInfo) => {
  await startCapture(page);
  const sort = page.getByRole("button", {
    name: "Sort collections",
    exact: true,
  });
  const refresh = page
    .getByRole("button", { name: "Refresh", exact: true })
    .first();
  const sortBox = (await sort.boundingBox())!;
  const refreshBox = (await refresh.boundingBox())!;
  expect(sortBox.y).toBe(refreshBox.y);
  expect(sortBox.height).toBe(refreshBox.height);
  const backBox = (await page
    .getByRole("button", { name: "Back", exact: true })
    .boundingBox())!;
  const forwardBox = (await page
    .getByRole("button", { name: "Forward", exact: true })
    .boundingBox())!;
  expect(refreshBox.x - (sortBox.x + sortBox.width)).toBe(
    forwardBox.x - (backBox.x + backBox.width)
  );
  const sidebarBox = (await page
    .getByRole("button", { name: "All items", exact: true })
    .boundingBox())!;
  await sort.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("menuitemradio", { name: "Sort by unread", exact: true })
  ).toHaveAttribute("aria-checked", "true");
  await testInfo.attach("sidebar-sort-menu", {
    body: await page.getByRole("menu").screenshot(),
    contentType: "image/png",
  });
  const menuBox = (await page.getByRole("menu").boundingBox())!;
  expect(menuBox.x).toBeGreaterThanOrEqual(sidebarBox.x + sidebarBox.width);
  expect(menuBox.x + menuBox.width).toBeLessThanOrEqual(
    page.viewportSize()!.width
  );
  await testInfo.attach("sidebar-toolbar-full", {
    body: await page.screenshot(),
    contentType: "image/png",
  });
  await page.keyboard.press("Escape");
  await expect(sort).toBeFocused();
  await page.getByRole("button", { name: "Add feed", exact: true }).click();
  const dialog = page.getByRole("dialog").filter({ hasText: "Add Feed" });
  await dialog
    .getByRole("textbox", { name: "URL", exact: true })
    .fill(additionalFeedUrl);
  await dialog
    .getByRole("textbox", { name: "Name (optional)", exact: true })
    .fill("A workshop");
  await dialog
    .getByRole("checkbox", { name: "Add to Shortcuts", exact: true })
    .check();
  await dialog.getByRole("button", { name: "Add", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await page.getByRole("button", { name: "Feeds (2)", exact: true }).click();
  const feedOrder = page
    .getByRole("list", { name: "Feeds", exact: true })
    .getByRole("button")
    .filter({ hasText: /^(Field notes|A workshop)$/ });
  await expect(feedOrder).toHaveText(["Field notes", "A workshop"]);
  await sort.focus();
  await page.keyboard.press("Enter");
  await page.keyboard.press("End");
  await page.keyboard.press("Enter");
  await expect(sort).toBeFocused();
  await expect(feedOrder).toHaveText(["A workshop", "Field notes"]);
  await page
    .getByRole("button", { name: "Shortcuts (2)", exact: true })
    .click();
  const shortcutOrder = page
    .getByRole("list", { name: "Shortcuts", exact: true })
    .getByRole("button")
    .filter({ hasText: /^(Everyday discoveries|A workshop)$/ });
  await expect(shortcutOrder).toHaveText([
    "A workshop",
    "Everyday discoveries",
  ]);
  // Reading changes unread counts without changing the shared choice.
  await page.getByRole("button", { name: "All items", exact: true }).click();
  await page.getByRole("button", { name: "All", exact: true }).click();
  await openArticle(page, "A garden for every season");
  await sort.click();
  await expect(
    page.getByRole("menuitemradio", { name: "Sort by name", exact: true })
  ).toHaveAttribute("aria-checked", "true");
  await page.keyboard.press("Escape");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await sort.click();
  await expect(
    page.getByRole("menuitemradio", { name: "Sort by name", exact: true })
  ).toHaveAttribute("aria-checked", "true");
  await page
    .getByRole("menuitemradio", { name: "Sort by unread", exact: true })
    .click();
});
