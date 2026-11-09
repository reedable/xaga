import {
  test,
  expect,
  startCapture,
  openArticle,
  articleButton,
  reader,
} from "./fixtures";

const garden = "A garden for every season";
const water = "Following the water";

test("Back and Forward restore Topic articles without restoring list filters", async ({
  page,
}) => {
  await startCapture(page);
  await openArticle(page, water);
  await page.getByRole("tab", { name: "Topics", exact: true }).click();
  await page
    .getByRole("checkbox", { name: "Everyday discoveries", exact: true })
    .check();
  await page.getByRole("button", { name: "Topics (1)", exact: true }).click();
  await page
    .getByRole("list", { name: "Topics", exact: true })
    .getByRole("button", { name: "Everyday discoveries", exact: true })
    .click();
  const unread = page.getByRole("button", { name: "Unread", exact: true });
  const all = page.getByRole("button", { name: "All", exact: true });
  const saved = page.getByRole("button", { name: "Saved", exact: true });
  await expect(unread).toHaveAttribute("aria-pressed", "true");
  await openArticle(page, garden);
  // Reading marks the article read but keeps the open list stable.
  await expect(articleButton(page, garden)).toBeVisible();
  const historyIndex = await page.evaluate(() => history.state.idx);
  await all.click();
  await expect(articleButton(page, water)).toBeVisible();
  expect(await page.evaluate(() => history.state.idx)).toBe(historyIndex);
  await openArticle(page, water);
  await saved.click();
  await expect(saved).toHaveAttribute("aria-pressed", "true");
  await expect(articleButton(page, garden)).toHaveCount(0);
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  await expect(saved).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Back", exact: true }).click();
  // Collection browsing did not add a step: this restores the earlier article in All items.
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    water
  );
  await unread.click();
  await expect(unread).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Forward", exact: true }).click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  await expect(unread).toHaveAttribute("aria-pressed", "true");
  await expect(articleButton(page, garden)).toHaveCount(0);
  await page.getByRole("button", { name: "Forward", exact: true }).click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    water
  );
  await expect(unread).toHaveAttribute("aria-pressed", "true");
  // The active filter also survives an ordinary collection change.
  await all.click();
  await page.getByRole("button", { name: "All items", exact: true }).click();
  await expect(all).toHaveAttribute("aria-pressed", "true");
});

test("only history restoration scrolls the selected entity into view", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 420 });
  await startCapture(page);
  await page.getByRole("button", { name: "All", exact: true }).click();
  await openArticle(page, garden);
  const wood = "Making room for a workshop";
  // Constrain the list so the small shared fixture requires scrolling.
  await page.addStyleTag({
    content: "[data-virtuoso-scroller] { max-height: 6rem; }",
  });
  const scroller = page.locator("[data-virtuoso-scroller]");
  await scroller.evaluate((node) => node.scrollTo({ top: node.scrollHeight }));
  const woodButton = articleButton(page, wood);
  await woodButton.scrollIntoViewIfNeeded();
  await woodButton.evaluate((node) => {
    node.addEventListener(
      "click",
      () => {
        document.documentElement.dataset.listScrollAtClick = String(
          node.closest("[data-virtuoso-scroller]")!.scrollTop
        );
      },
      { once: true }
    );
  });
  await openArticle(page, wood);
  const atClick = await page
    .locator("html")
    .getAttribute("data-list-scroll-at-click");
  await expect(scroller).toHaveJSProperty("scrollTop", Number(atClick));
  expect(Number(atClick)).toBeGreaterThan(0);
  const back = page.getByRole("button", { name: "Back", exact: true });
  // Leave another Forward destination so the button stays enabled for the focus check.
  await scroller.evaluate((node) => node.scrollTo({ top: 0 }));
  await openArticle(page, water);
  await back.click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    wood
  );
  await back.click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  await expect(articleButton(page, garden)).toBeInViewport();
  await expect(back).toBeFocused();
  await expect(scroller).toHaveJSProperty("scrollTop", 0);
  const forward = page.getByRole("button", { name: "Forward", exact: true });
  await forward.click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    wood
  );
  await expect(woodButton).toBeInViewport();
  await expect(forward).toBeFocused();
});

test("collection browsing leaves article history and saved collection snapshots untouched", async ({
  page,
}) => {
  await startCapture(page);
  await page.getByRole("button", { name: "All", exact: true }).click();
  await openArticle(page, garden);
  const gardenHistory = await page.evaluate(() =>
    structuredClone(history.state)
  );
  await page.getByRole("button", { name: "Topics (1)", exact: true }).click();
  await page
    .getByRole("list", { name: "Topics", exact: true })
    .getByRole("button", { name: "Everyday discoveries", exact: true })
    .click();
  expect(await page.evaluate(() => history.state)).toEqual(gardenHistory);
  await page.getByRole("button", { name: "Feeds (1)", exact: true }).click();
  await page
    .getByRole("list", { name: "Feeds", exact: true })
    .getByRole("button", { name: "Field notes", exact: true })
    .click();
  expect(await page.evaluate(() => history.state)).toEqual(gardenHistory);
  await openArticle(page, water);
  const waterHistory = await page.evaluate(() =>
    structuredClone(history.state)
  );
  expect(waterHistory.idx).toBe(gardenHistory.idx + 1);
  await page.getByRole("button", { name: "All items", exact: true }).click();
  expect(await page.evaluate(() => history.state)).toEqual(waterHistory);
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  await expect(articleButton(page, "Making room for a workshop")).toBeVisible();
  await page.getByRole("button", { name: "Topics (1)", exact: true }).click();
  await page
    .getByRole("list", { name: "Topics", exact: true })
    .getByRole("button", { name: "Everyday discoveries", exact: true })
    .click();
  await page.getByRole("button", { name: "Forward", exact: true }).click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    water
  );
  // Water was opened from Field notes, so the workshop article is excluded.
  await expect(articleButton(page, "Making room for a workshop")).toHaveCount(
    0
  );
  await expect(
    page.getByText("Field notes", { exact: true }).filter({ visible: true })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "All", exact: true })
  ).toHaveAttribute("aria-pressed", "true");
});
