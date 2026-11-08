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
const passage = /^Small changes/;

for (const scope of ["article", "all", "topic", "feed"] as const) {
  test(`annotation edits preserve selection and context in ${scope} list`, async ({
    page,
  }) => {
    await startCapture(page);
    await openArticle(page, garden);
    await reader(page).locator("article p").first().click({ clickCount: 3 });
    await page.getByRole("button", { name: "violet", exact: true }).click();
    if (scope === "topic") {
      await page
        .getByRole("button", { name: "Show annotation details" })
        .click();
      await page.getByRole("tab", { name: "Topics", exact: true }).click();
      await page
        .getByRole("checkbox", { name: "Everyday discoveries", exact: true })
        .check();
      await page
        .getByRole("button", { name: "Close details", exact: true })
        .click();
    }
    if (scope === "article") {
      await shareAction(page, "Add to Shortcuts");
      await page.getByRole("button", { name: garden, exact: true }).click();
    } else if (scope === "topic") {
      await page
        .getByRole("button", { name: "Topics (1)", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Everyday discoveries", exact: true })
        .first()
        .click();
      await page.getByRole("button", { name: "All", exact: true }).click();
    } else if (scope === "feed") {
      await page
        .getByRole("button", { name: "Feeds (1)", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Field notes", exact: true })
        .click();
      await page.getByRole("button", { name: "All", exact: true }).click();
    } else {
      await page.getByRole("button", { name: "All", exact: true }).click();
    }
    const annotation = page.getByRole("button", { name: passage });
    if (scope !== "feed") {
      await annotation.click();
      await expect(annotation).toHaveClass(/Mui-selected/);
    } else {
      // Feed lists contain articles; edit an unselected annotation in the reader.
      await articleButton(page, garden).click();
    }
    const listButton =
      scope === "feed" ? articleButton(page, garden) : annotation;
    const membersBefore = await page
      .locator("[data-virtuoso-scroller] .MuiListItemButton-root")
      .allTextContents();
    await reader(page)
      .locator("mark")
      .filter({ hasText: "Small changes" })
      .click();
    await page.getByRole("button", { name: "blue", exact: true }).click();
    await expect(listButton).toHaveClass(/Mui-selected/);
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      garden
    );
    await expect(
      page.locator("[data-virtuoso-scroller] .MuiListItemButton-root")
    ).toHaveText(membersBefore);
    await reader(page)
      .locator("mark")
      .filter({ hasText: "Small changes" })
      .click();
    await page
      .getByRole("button", { name: "Remove marking", exact: true })
      .click();
    await page.getByRole("button", { name: "Remove", exact: true }).click();
    await expect(reader(page).locator("mark")).toHaveCount(0);
    await expect(articleButton(page, garden)).toHaveClass(/Mui-selected/);
    if (scope !== "feed") await expect(annotation).toHaveCount(0);
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      garden
    );
    if (scope !== "article")
      await expect(
        page.getByRole("button", { name: "All", exact: true })
      ).toHaveAttribute("aria-pressed", "true");
  });
}
