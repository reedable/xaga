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
    const refresh = page
      .getByRole("button", { name: "Refresh", exact: true })
      .first();
    await expect(refresh).toBeDisabled();
    await refresh.locator("..").hover();
    await expect(page.getByRole("tooltip")).toContainText("read-only");
    await page.mouse.move(1400, 850);
    await openArticle(page, "A garden for every season");
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      "A garden for every season"
    );
    await expect(
      page.getByRole("button", { name: "Add bookmark", exact: true })
    ).toBeDisabled();
    await expect(
      page.getByRole("button", { name: "Mark all read", exact: true })
    ).toBeDisabled();
    await page.getByRole("button", { name: "Share", exact: true }).click();
    await expect(
      page.getByRole("menuitem", { name: "Mark as read", exact: true })
    ).toBeDisabled();
    await expect(
      page.getByRole("menuitem", { name: /Open in default browser/i })
    ).toBeEnabled();
    await page.keyboard.press("Escape");
    await page
      .getByRole("button", { name: "Sort collections", exact: true })
      .click();
    await expect(
      page.getByRole("menuitemradio", { name: "Sort by name", exact: true })
    ).toBeEnabled();
  });
}
