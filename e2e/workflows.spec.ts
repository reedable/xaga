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

test("read status updates Unread membership and collection counts", async ({
  page,
}) => {
  const allItems = page.getByRole("listitem").filter({
    has: page.getByRole("button", { name: "All items", exact: true }),
  });
  await expect(allItems).toHaveText("All items2");
  await openArticle(page, garden);
  await expect(allItems).toHaveText("All items1");
  await shareAction(page, "Mark as unread");
  await expect(allItems).toHaveText("All items2");
  await shareAction(page, "Mark as read");
  await expect(allItems).toHaveText("All items1");
  // The open list intentionally stays stable until navigation reloads membership.
  await page.getByRole("button", { name: "All", exact: true }).click();
  await page.getByRole("button", { name: "Unread", exact: true }).click();
  await expect(articleButton(page, garden)).toHaveCount(0);
  await expect(articleButton(page, water)).toBeVisible();
  await page.getByRole("button", { name: "Feeds (1)", exact: true }).click();
  const feed = page.getByRole("listitem").filter({
    has: page.getByRole("button", { name: "Field notes", exact: true }),
  });
  await expect(feed).toHaveText("Field notes1");
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
});

test("a colored bookmark appears in Saved and can be removed", async ({
  page,
}) => {
  await openArticle(page, garden);
  await page.getByRole("button", { name: "Add bookmark", exact: true }).click();
  const palette = page.getByRole("dialog", { name: "Bookmark color" });
  await palette.getByRole("button", { name: "blue", exact: true }).click();
  await page.getByRole("button", { name: "Saved", exact: true }).click();
  await openArticle(page, garden);
  await page
    .getByRole("button", { name: "Change bookmark color", exact: true })
    .click();
  await expect(
    palette.getByRole("button", { name: "blue", exact: true })
  ).toHaveAttribute("aria-pressed", "true");
  await palette
    .getByRole("button", { name: "Remove marking", exact: true })
    .click();
  await palette.getByRole("button", { name: "Remove", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Add bookmark", exact: true })
  ).toBeEnabled();
  await page.getByRole("button", { name: "All", exact: true }).click();
  await page.getByRole("button", { name: "Saved", exact: true }).click();
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
  await page.getByRole("button", { name: "violet", exact: true }).click();
  await page.getByPlaceholder("Add a note…").fill("Return to this idea");
  await page.getByRole("button", { name: "Saved", exact: true }).click();
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
  await expect(page.getByPlaceholder("Add a note…")).toHaveValue(
    "Return to this idea"
  );
  await expect(
    page.getByRole("button", { name: "violet", exact: true })
  ).toHaveAttribute("aria-pressed", "true");
});

test("Topic membership is visible when browsing with the sidebar hidden", async ({
  page,
}) => {
  await openArticle(page, water);
  await page.getByRole("tab", { name: "Topics", exact: true }).click();
  await page
    .getByRole("checkbox", { name: "Everyday discoveries", exact: true })
    .check();
  await expect(
    page.getByRole("checkbox", { name: "Everyday discoveries", exact: true })
  ).toBeChecked();
  // Responsive layout hides the sidebar; the collection selector is the real UI.
  await page.setViewportSize({ width: 960, height: 900 });
  await expect(
    page.getByRole("textbox", { name: "Filter collections", exact: true })
  ).not.toBeVisible();
  await page
    .getByRole("combobox", { name: "Browse collections and tags" })
    .click();
  await page
    .getByRole("option", { name: "Everyday discoveries", exact: true })
    .first()
    .click();
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "All", exact: true }).click();
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
  await shareAction(page, "Add to Shortcuts");
  await expect(
    page.getByRole("button", { name: "Shortcuts (2)", exact: true })
  ).toBeVisible();
  const shortcut = page.getByRole("button", { name: garden, exact: true });
  await expect(shortcut).toBeVisible();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  await expect(articleButton(page, garden)).toBeVisible();
  await expect(articleButton(page, water)).toBeVisible();
  await shareAction(page, "Remove from Shortcuts");
  await expect(shortcut).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Shortcuts (1)", exact: true })
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
  const initialUnreadSelection = await page
    .getByRole("button", { name: "Unread", exact: true })
    .getAttribute("aria-pressed");
  await openArticle(page, garden);
  await shareAction(page, "Add to Shortcuts");
  await page.getByRole("button", { name: "Add bookmark", exact: true }).click();
  await page
    .getByRole("dialog", { name: "Bookmark color" })
    .getByRole("button", { name: "blue", exact: true })
    .click();
  await page.getByRole("button", { name: "Saved", exact: true }).click();
  await page.getByRole("button", { name: "Feeds (1)", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Filter collections", exact: true })
    .fill("Field");
  await page
    .getByRole("textbox", { name: "Filter items…", exact: true })
    .fill("garden");
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
  await expect(
    page.getByText("Select an article", { exact: true })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Shortcuts (1)", exact: true })
  ).toHaveAttribute("aria-pressed", "true");
  await expect(
    page.getByRole("button", { name: "Unread", exact: true })
  ).toHaveAttribute("aria-pressed", initialUnreadSelection!);
  await expect(
    page.getByRole("textbox", { name: "Filter collections", exact: true })
  ).toBeEmpty();
  await expect(
    page.getByRole("textbox", { name: "Filter items…", exact: true })
  ).toBeEmpty();
  await expect(articleButton(page, garden)).toBeVisible();
  await expect(articleButton(page, water)).toBeVisible();
  await page.getByRole("button", { name: "Saved", exact: true }).click();
  await expect(articleButton(page, garden)).toHaveCount(0);
});
