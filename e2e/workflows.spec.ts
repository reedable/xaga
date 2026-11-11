import { control } from "./fixtures";
import {
  test,
  expect,
  startCapture,
  articleButton,
  openArticle,
  reader,
  shareAction,
} from "./fixtures";

const garden = "A garden for every season";
const water = "Following the water";

// Each test receives Playwright's fresh context and a new in-memory capture app.
test.beforeEach(async ({ page }) => {
  await startCapture(page);
});

test("All items stays above the collection filter across section changes", async ({
  page,
}) => {
  const allItems = control(page, "button", "all-items");
  const filter = control(page, "textbox", "filter-collections");
  await expect(allItems).toHaveCount(1);
  const itemBox = await allItems.boundingBox();
  const filterBox = await filter.boundingBox();
  expect(itemBox!.y + itemBox!.height).toBeLessThan(filterBox!.y);
  for (const section of ["shortcuts", "topics", "feeds"]) {
    await control(page, "button", `sidebar-${section}-toggle`).click();
    await expect(allItems).toBeVisible();
  }
  await filter.fill("no matching collection");
  await expect(allItems).toBeVisible();
  await allItems.focus();
  await page.keyboard.press("Enter");
  await expect(allItems).toHaveClass(/Mui-selected/);
  await expect(articleButton(page, water)).toBeVisible();
  await expect(filter).toHaveValue("no matching collection");
});

test("collapsing sidebar rows cannot compete with the newly opened section", async ({
  page,
}) => {
  // Keep the outgoing rows mounted through a deliberately slow transition.
  await page.addStyleTag({
    content: ".MuiCollapse-root { transition-duration: 10s !important; }",
  });
  const shortcuts = control(page, "list", "sidebar-shortcuts-list", true);
  await control(page, "button", "sidebar-topics-toggle").click();
  const outgoingTopic = shortcuts.getByRole("button", {
    name: "Everyday discoveries",
    exact: true,
    includeHidden: true,
  });
  await expect(outgoingTopic).toHaveCount(1);
  expect(
    await shortcuts.evaluate(
      (node) => node.closest('[inert][aria-hidden="true"]') !== null
    )
  ).toBe(true);
  await outgoingTopic.evaluate((node) => {
    (node as HTMLElement).focus();
  });
  await expect(control(page, "button", "sidebar-topics-toggle")).toBeFocused();
  // Intentionally unscoped: the outgoing shortcut must no longer match.
  const topic = page.getByRole("button", {
    name: "Everyday discoveries",
    exact: true,
  });
  await expect(topic).toHaveCount(1);
  await topic.click();
  await expect(topic).toHaveClass(/Mui-selected/);
  await control(page, "button", "sidebar-shortcuts-toggle").click();
  await expect(topic).toHaveCount(1);
  await topic.click();
  await expect(topic).toHaveClass(/Mui-selected/);
});

test("read status updates Unread membership and collection counts", async ({
  page,
}) => {
  const allItems = page.getByRole("listitem").filter({
    has: control(page, "button", "all-items"),
  });
  await expect(allItems.locator("[data-unread-count]")).toHaveAttribute(
    "data-unread-count",
    "2"
  );
  await openArticle(page, garden);
  await expect(allItems.locator("[data-unread-count]")).toHaveAttribute(
    "data-unread-count",
    "1"
  );
  await shareAction(page, "toggle-read");
  await expect(allItems.locator("[data-unread-count]")).toHaveAttribute(
    "data-unread-count",
    "2"
  );
  await shareAction(page, "toggle-read");
  await expect(allItems.locator("[data-unread-count]")).toHaveAttribute(
    "data-unread-count",
    "1"
  );
  // The open list intentionally stays stable until navigation reloads membership.
  await control(page, "button", "filter-all").click();
  await control(page, "button", "filter-unread").click();
  await expect(articleButton(page, garden)).toHaveCount(0);
  await expect(articleButton(page, water)).toBeVisible();
  await control(page, "button", "sidebar-feeds-toggle").click();
  const feed = control(page, "list", "sidebar-feeds-list")
    .getByRole("listitem")
    .filter({
      has: page.getByRole("button", { name: "Field notes", exact: true }),
    });
  await expect(feed.locator("[data-unread-count]")).toHaveAttribute(
    "data-unread-count",
    "1"
  );
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
});

test("a colored bookmark appears in Saved and can be removed", async ({
  page,
}) => {
  await openArticle(page, garden);
  const unmarkedLabel = await articleButton(page, garden).ariaSnapshot();
  await control(page, "button", "bookmark-toggle").click();
  const palette = control(page, "dialog", "bookmark-palette");
  await control(palette, "button", "blue").click();
  await expect(articleButton(page, garden)).toHaveAttribute(
    "data-bookmark-color",
    "blue"
  );
  await expect(articleButton(page, water)).toHaveAttribute(
    "data-bookmark-color",
    ""
  );
  await expect(articleButton(page, garden)).toHaveAccessibleName(/\S/);
  await expect
    .poll(() => articleButton(page, garden).ariaSnapshot())
    .not.toBe(unmarkedLabel);
  // Marked and unmarked article titles retain the same horizontal alignment.
  const titleLeft = async (title: string) =>
    articleButton(page, title)
      .locator(".MuiListItemText-primary")
      .evaluate((node) => node.getBoundingClientRect().left);
  expect(await titleLeft(garden)).toBe(await titleLeft(water));
  await expect(
    articleButton(page, garden).locator('[data-testid="BookmarkIcon"]')
  ).toHaveCount(0);
  const saved = control(page, "button", "filter-saved");
  await saved.click();
  await expect(saved).toHaveAttribute("aria-pressed", "true");
  await expect(articleButton(page, water)).toHaveCount(0);
  await openArticle(page, garden);
  const blueLabel = await articleButton(page, garden).ariaSnapshot();
  await control(page, "button", "bookmark-toggle").click();
  await expect(control(palette, "button", "blue")).toHaveAttribute(
    "aria-pressed",
    "true"
  );
  await control(palette, "button", "red").click();
  await expect(articleButton(page, garden)).toHaveAttribute(
    "data-bookmark-color",
    "red"
  );
  await expect
    .poll(() => articleButton(page, garden).ariaSnapshot())
    .not.toBe(blueLabel);
  await control(page, "button", "bookmark-toggle").click();
  await control(palette, "button", "remove-marking").click();
  await control(palette, "button", "confirm-remove").click();
  await expect(control(page, "button", "bookmark-toggle")).toBeEnabled();
  await expect(articleButton(page, garden)).toBeVisible();
  await expect(articleButton(page, garden)).toHaveAttribute(
    "data-bookmark-color",
    ""
  );
  await expect
    .poll(() => articleButton(page, garden).ariaSnapshot())
    .toBe(unmarkedLabel);
  // Bookmark edits preserve the open list. Wait for All to finish navigating
  // before returning to Saved; otherwise the second click can hit the old view.
  const all = control(page, "button", "filter-all");
  await all.click();
  await expect(all).toHaveAttribute("aria-pressed", "true");
  await expect(articleButton(page, water)).toBeVisible();
  await saved.click();
  await expect(saved).toHaveAttribute("aria-pressed", "true");
  await expect(articleButton(page, garden)).toHaveCount(0);
  await expect(articleButton(page, "Making room for a workshop")).toBeVisible();
});

test("an annotation can be created and its passage revisited from Saved", async ({
  page,
}) => {
  await openArticle(page, garden);
  const paragraph = reader(page).locator("article p").first();
  // A real triple click selects a paragraph and sends the reader's mouseup.
  await paragraph.click({ clickCount: 3 });
  await control(page, "button", "violet").click();
  await control(page, "textbox", "annotation-comment").fill(
    "Return to this idea"
  );
  await control(page, "button", "filter-saved").click();
  const annotation = page.getByRole("button", { name: /^Small changes/ });
  await expect(annotation).toBeVisible();
  // Leave the article before revisiting the saved annotation.
  await openArticle(page, "Making room for a workshop");
  await annotation.click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  const highlight = reader(page)
    .locator("mark")
    .filter({ hasText: "Small changes" });
  await expect(highlight).toBeInViewport();
  await highlight.click();
  await expect(control(page, "textbox", "annotation-comment")).toHaveValue(
    "Return to this idea"
  );
  await expect(control(page, "button", "violet")).toHaveAttribute(
    "aria-pressed",
    "true"
  );
});

test("Topic membership is visible when browsing with the sidebar hidden", async ({
  page,
}) => {
  await openArticle(page, water);
  await control(page, "tab", "inspector-topics").click();
  await page
    .getByRole("checkbox", { name: "Everyday discoveries", exact: true })
    .check();
  await expect(
    page.getByRole("checkbox", { name: "Everyday discoveries", exact: true })
  ).toBeChecked();
  // Responsive layout hides the sidebar; the collection selector is the real UI.
  await page.setViewportSize({ width: 960, height: 900 });
  await expect(
    control(page, "textbox", "filter-collections")
  ).not.toBeVisible();
  await control(page, "combobox", "collection-selector").click();
  await page
    .getByRole("option", { name: "Everyday discoveries", exact: true })
    .first()
    .click();
  await page.keyboard.press("Escape");
  await control(page, "button", "filter-all").click();
  await expect(articleButton(page, water)).toBeVisible();
  await expect(articleButton(page, garden)).toBeVisible();
  await articleButton(page, garden).focus();
  await page.keyboard.press("Enter");
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
});

test("pinning and unpinning updates Shortcuts without clearing the reading context", async ({
  page,
}) => {
  await openArticle(page, garden);
  await shareAction(page, "pin-toggle");
  await expect(
    control(page, "button", "sidebar-shortcuts-toggle")
  ).toBeVisible();
  const shortcut = page.getByRole("button", { name: garden, exact: true });
  await expect(shortcut).toBeVisible();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  await expect(articleButton(page, garden)).toBeVisible();
  await expect(articleButton(page, water)).toBeVisible();
  await shareAction(page, "pin-toggle");
  await expect(shortcut).toHaveCount(0);
  await expect(
    control(page, "button", "sidebar-shortcuts-toggle")
  ).toBeVisible();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  // The list's arrow-key navigation still works after both shortcut edits.
  await articleButton(page, garden).focus();
  await page.keyboard.press("ArrowDown");
  await expect(articleButton(page, water)).toBeFocused();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    water
  );
});

test("capture reset clears edits, navigation, filters, and expansion", async ({
  page,
}) => {
  const initialUnreadSelection = await control(
    page,
    "button",
    "filter-unread"
  ).getAttribute("aria-pressed");
  await openArticle(page, garden);
  await shareAction(page, "pin-toggle");
  await control(page, "button", "bookmark-toggle").click();
  await control(page, "dialog", "bookmark-palette")
    .locator('[data-color="blue"]')
    .click();
  await control(page, "button", "filter-saved").click();
  await control(page, "button", "sidebar-feeds-toggle").click();
  await control(page, "textbox", "filter-collections").fill("Field");
  await control(page, "textbox", "filter-items").fill("garden");
  await Promise.all([
    page.waitForEvent("load"),
    page.evaluate(() =>
      (
        window as Window & { xagaCapture?: { reset(): void } }
      ).xagaCapture!.reset()
    ),
  ]);
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await expect(page.getByTestId("reader-empty")).toBeVisible();
  await expect(
    control(page, "button", "sidebar-shortcuts-toggle")
  ).toHaveAttribute("aria-pressed", "true");
  await expect(control(page, "button", "filter-unread")).toHaveAttribute(
    "aria-pressed",
    initialUnreadSelection!
  );
  await expect(control(page, "textbox", "filter-collections")).toBeEmpty();
  await expect(control(page, "textbox", "filter-items")).toBeEmpty();
  await expect(articleButton(page, garden)).toBeVisible();
  await expect(articleButton(page, water)).toBeVisible();
  await control(page, "button", "filter-saved").click();
  await expect(articleButton(page, garden)).toHaveCount(0);
});
