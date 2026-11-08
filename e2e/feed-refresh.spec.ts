import { test, expect } from "./fixtures";
for (const scenario of [
  "refresh-failure",
  "refresh-all-fail",
  "refresh-refused",
  "startup-refresh-failure",
]) {
  test(`${scenario}: shared error and retry cleanup`, async ({ page }) => {
    await page.goto(`/?scenario=${scenario}`);
    await expect(page.locator("html")).toHaveAttribute(
      "data-capture-ready",
      "true"
    );
    const refresh = page
      .getByRole("button", { name: "Refresh", exact: true })
      .first();
    if (scenario !== "startup-refresh-failure") await refresh.click();
    const alert = page.getByRole("alert");
    await expect(alert).toContainText("Use Refresh to retry.");
    await expect(alert).toContainText(
      scenario === "refresh-refused" ? "read-only" : "Field notes"
    );
    await expect(refresh).toBeEnabled();
    await alert.getByRole("button", { name: "Close" }).click();
    await refresh.click();
    await expect(refresh).toBeEnabled();
    if (scenario !== "refresh-refused")
      await expect(page.getByRole("alert")).toHaveCount(0);
  });
}
test("no-new-article refresh succeeds without an error", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  const refresh = page
    .getByRole("button", { name: "Refresh", exact: true })
    .first();
  await refresh.click();
  await expect(refresh).toBeEnabled();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: /^A garden for every season / })
  ).toBeVisible();
});
