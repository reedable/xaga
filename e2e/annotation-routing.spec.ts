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
const passage = /^Small changes/;

for (const scope of ["article", "all", "topic", "feed"] as const) {
  test(`annotation edits preserve selection and context in ${scope} list`, async ({
    page,
  }) => {
    // This full create → browse → recolor → remove workflow exceeds 30 seconds
    // under CPU throttling. Keep the individual assertion deadlines unchanged.
    test.setTimeout(60_000);
    await startCapture(page);
    await openArticle(page, garden);
    await reader(page).locator("article p").first().click({ clickCount: 3 });
    await control(page, "button", "violet").click();
    if (scope === "topic") {
      await control(page, "button", "annotation-details").click();
      await control(page, "tab", "inspector-topics").click();
      await page
        .getByRole("checkbox", { name: "Everyday discoveries", exact: true })
        .check();
      await control(page, "button", "close-details").click();
    }
    if (scope === "article") {
      await shareAction(page, "pin-toggle");
      await page.getByRole("button", { name: garden, exact: true }).click();
    } else if (scope === "topic") {
      await control(page, "button", "sidebar-topics-toggle").click();
      await control(page, "list", "sidebar-topics-list")
        .getByRole("button", { name: "Everyday discoveries", exact: true })
        .click();
      await control(page, "button", "filter-all").click();
    } else if (scope === "feed") {
      await control(page, "button", "sidebar-feeds-toggle").click();
      await control(page, "list", "sidebar-feeds-list")
        .getByRole("button", { name: "Field notes", exact: true })
        .click();
      await control(page, "button", "filter-all").click();
    } else {
      await control(page, "button", "filter-all").click();
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
      .getByTestId("entity-list-scroller")
      .locator(".MuiListItemButton-root")
      .allTextContents();
    await reader(page)
      .locator("mark")
      .filter({ hasText: "Small changes" })
      .click();
    await control(page, "button", "blue").click();
    await expect(listButton).toHaveClass(/Mui-selected/);
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      garden
    );
    await expect(
      page
        .getByTestId("entity-list-scroller")
        .locator(".MuiListItemButton-root")
    ).toHaveText(membersBefore);
    await reader(page)
      .locator("mark")
      .filter({ hasText: "Small changes" })
      .click();
    await control(page, "button", "remove-marking").click();
    await control(page, "button", "confirm-remove").click();
    await expect(reader(page).locator("mark")).toHaveCount(0);
    await expect(articleButton(page, garden)).toHaveClass(/Mui-selected/);
    if (scope !== "feed") await expect(annotation).toHaveCount(0);
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      garden
    );
    if (scope !== "article")
      await expect(control(page, "button", "filter-all")).toHaveAttribute(
        "aria-pressed",
        "true"
      );
  });
}
