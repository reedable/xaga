import { control } from "./fixtures";
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
    const refresh = control(page, "button", "refresh").first();
    if (scenario !== "startup-refresh-failure") await refresh.click();
    const alert = page.getByRole("alert");
    await expect(alert).toBeVisible();
    await expect(alert).not.toBeEmpty();
    await expect(refresh).toBeEnabled();
    await control(alert, "button", "dismiss-notification").click();
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
  const refresh = control(page, "button", "refresh").first();
  await refresh.click();
  await expect(refresh).toBeEnabled();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: /^A garden for every season / })
  ).toBeVisible();
});

for (const theme of ["light", "dark"] as const) {
  test(`many feed failures have scrollable details in ${theme}`, async ({
    page,
  }, testInfo) => {
    await page.emulateMedia({ colorScheme: theme });
    await page.goto(`/?scenario=refresh-many-failures&theme=${theme}`);
    await expect(page.locator("html")).toHaveAttribute(
      "data-capture-ready",
      "true"
    );
    const refresh = control(page, "button", "refresh").first();
    await refresh.click();
    const alert = page.getByRole("alert");
    await expect(alert).toBeVisible();
    expect((await alert.boundingBox())!.height).toBeLessThan(100);
    await control(alert, "button", "feed-refresh-details").click();
    const dialog = page.getByRole("dialog");
    const background = await dialog.evaluate(
      (node) => getComputedStyle(node).backgroundColor
    );
    const channels = background
      .match(/[\d.]+/g)!
      .slice(0, 3)
      .map(Number);
    if (theme === "dark") expect(Math.max(...channels)).toBeLessThan(128);
    else expect(Math.min(...channels)).toBeGreaterThan(128);
    const rows = dialog.locator("li[data-feed-id]");
    await expect(rows).toHaveCount(14);
    const content = dialog.locator(".MuiDialogContent-root");
    expect(
      await content.evaluate((node) => node.scrollHeight > node.clientHeight)
    ).toBe(true);
    await rows.last().scrollIntoViewIfNeeded();
    await expect(rows.last()).toBeInViewport();
    await expect(rows.last().getByRole("heading")).toHaveAccessibleName(/\S/);
    await page.screenshot({
      path: testInfo.outputPath(`feed-failures-${theme}.png`),
      animations: "disabled",
    });
    await control(dialog, "button", "close-feed-failures").click();
    await expect(alert).toBeVisible();
    await control(alert, "button", "dismiss-notification").click();
    await refresh.click();
    await expect(refresh).toBeEnabled();
    await expect(alert).toHaveCount(0);
  });
}
