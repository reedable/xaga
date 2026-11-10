import { control } from "./fixtures";
import { test, expect, openArticle, reader } from "./fixtures";
for (const scenario of ["read-only", "read-only-delayed"]) {
  test(`${scenario}: browsing remains available while library writes are disabled`, async ({
    page,
  }) => {
    await page.goto(`/?scenario=${scenario}`);
    await expect(page.locator("html")).toHaveAttribute(
      "data-capture-ready",
      "true"
    );
    const refresh = control(page, "button", "refresh").first();
    await expect(refresh).toBeDisabled();
    await refresh.locator("..").hover();
    await expect(page.getByRole("tooltip")).toBeVisible();
    await page.mouse.move(1400, 850);
    await openArticle(page, "A garden for every season");
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      "A garden for every season"
    );
    await expect(control(page, "button", "bookmark-toggle")).toBeDisabled();
    await expect(control(page, "button", "mark-all-read")).toBeDisabled();
    await control(page, "button", "share").click();
    await expect(control(page, "menuitem", "toggle-read")).toBeDisabled();
    await expect(control(page, "menuitem", "open-browser")).toBeEnabled();
    await page.keyboard.press("Escape");
    await control(page, "button", "sidebar-sort-trigger").click();
    await expect(
      control(page, "menuitemradio", "sort-alphabetical")
    ).toBeEnabled();
  });
}
