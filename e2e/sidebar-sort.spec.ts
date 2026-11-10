import { control } from "./fixtures";
import { test, expect, startCapture, openArticle } from "./fixtures";
const additionalFeedUrl = "https://workshop.example/feed.xml";

test("one keyboard-accessible sidebar sort choice applies across sections and survives restart", async ({
  page,
}, testInfo) => {
  await startCapture(page);
  const sort = control(page, "button", "sidebar-sort-trigger");
  const refresh = control(page, "button", "refresh").first();
  const sortBox = (await sort.boundingBox())!;
  const refreshBox = (await refresh.boundingBox())!;
  expect(sortBox.y).toBe(refreshBox.y);
  expect(sortBox.height).toBe(refreshBox.height);
  const backBox = (await control(page, "button", "back").boundingBox())!;
  const forwardBox = (await control(page, "button", "forward").boundingBox())!;
  expect(refreshBox.x - (sortBox.x + sortBox.width)).toBe(
    forwardBox.x - (backBox.x + backBox.width)
  );
  const sidebarBox = (await control(
    page,
    "button",
    "all-items"
  ).boundingBox())!;
  await sort.focus();
  await page.keyboard.press("Enter");
  await expect(control(page, "menuitemradio", "sort-unread")).toHaveAttribute(
    "aria-checked",
    "true"
  );
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
  await control(page, "button", "add-feed").click();
  const dialog = page.getByRole("dialog").filter({ hasText: "Add Feed" });
  await control(dialog, "textbox", "feed-url").fill(additionalFeedUrl);
  await control(dialog, "textbox", "collection-name").fill("A workshop");
  await control(dialog, "checkbox", "pin-toggle").check();
  await control(dialog, "button", "add").click();
  await expect(dialog).not.toBeVisible();
  await control(page, "button", "sidebar-feeds-toggle").click();
  const feedOrder = control(page, "list", "sidebar-feeds-list")
    .getByRole("button")
    .filter({ hasText: /^(Field notes|A workshop)$/ });
  await expect(feedOrder).toHaveText(["Field notes", "A workshop"]);
  await sort.focus();
  await page.keyboard.press("Enter");
  await page.keyboard.press("End");
  await page.keyboard.press("Enter");
  await expect(sort).toBeFocused();
  await expect(feedOrder).toHaveText(["A workshop", "Field notes"]);
  await control(page, "button", "sidebar-shortcuts-toggle").click();
  const shortcutOrder = control(page, "list", "sidebar-shortcuts-list")
    .getByRole("button")
    .filter({ hasText: /^(Everyday discoveries|A workshop)$/ });
  await expect(shortcutOrder).toHaveText([
    "A workshop",
    "Everyday discoveries",
  ]);
  // Reading changes unread counts without changing the shared choice.
  await control(page, "button", "all-items").click();
  await control(page, "button", "filter-all").click();
  await openArticle(page, "A garden for every season");
  await sort.click();
  await expect(
    control(page, "menuitemradio", "sort-alphabetical")
  ).toHaveAttribute("aria-checked", "true");
  await page.keyboard.press("Escape");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await sort.click();
  await expect(
    control(page, "menuitemradio", "sort-alphabetical")
  ).toHaveAttribute("aria-checked", "true");
  await control(page, "menuitemradio", "sort-unread").click();
});
