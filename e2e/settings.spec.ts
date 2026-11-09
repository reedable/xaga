import { test, expect } from "./fixtures";

test("Settings exposes fetch controls, preserves exact values, and retains panel organization", async ({
  page,
}) => {
  await page.goto("/?window=settings");
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await expect(
    page.getByRole("button", { name: "Switch library", exact: true })
  ).toBeVisible();
  await expect(
    page.getByRole("checkbox", { name: "Show PDFs in the reading pane" })
  ).toHaveAccessibleDescription("When off, PDFs open in your default app.");
  await expect(
    page.getByRole("checkbox", { name: "Refresh all feeds on start-up" })
  ).toHaveAccessibleDescription(
    "Contacts subscribed websites automatically on the next startup."
  );
  const timeout = page.getByRole("textbox", {
    name: "Article load timeout",
    exact: true,
  });
  await expect(timeout).toHaveValue("10");
  await timeout.fill("9007199254740993");
  await timeout.press("Enter");
  await expect(timeout).toBeEnabled();
  await page.getByRole("button", { name: "Security", exact: true }).click();
  const http = page.getByRole("switch", {
    name: "Allow unencrypted HTTP",
    exact: true,
  });
  await expect(http).not.toBeChecked();
  await http.check();
  await expect(http).toBeChecked();
  await expect(
    page.getByRole("switch", { name: "Allow HTTPS connections on other ports" })
  ).not.toBeChecked();
  const redirects = page.getByRole("textbox", {
    name: "Maximum feed redirects",
    exact: true,
  });
  await expect(redirects).toHaveValue("3");
  await redirects.fill("11");
  await redirects.press("Enter");
  await expect(redirects).toHaveAttribute("aria-invalid", "true");
  await redirects.fill("0");
  await redirects.press("Enter");
  await expect(redirects).toHaveAttribute("aria-invalid", "false");
  await page.getByRole("button", { name: "Privacy", exact: true }).click();
  await expect(
    page.getByText("Keep website sessions", { exact: true })
  ).toBeVisible();
  await expect(
    page.getByRole("switch", { name: "Send referring URLs for feed redirects" })
  ).not.toBeChecked();
  await page
    .getByRole("switch", { name: "Allow compressed feed transfers" })
    .check();
  await page.getByRole("button", { name: "General", exact: true }).click();
  await expect(timeout).toHaveValue("9007199254740993");
  await expect(
    page.getByRole("textbox", {
      name: "Maximum feed response size",
      exact: true,
    })
  ).toHaveValue("10");
  const feedLimit = page.getByRole("textbox", {
    name: "Maximum feed response size",
    exact: true,
  });
  await feedLimit.fill("12.5");
  await feedLimit.press("Enter");
  await page.getByRole("button", { name: "Security", exact: true }).click();
  await page.getByRole("button", { name: "General", exact: true }).click();
  await expect(feedLimit).toHaveValue("12.5");
  await page.getByRole("button", { name: "Appearance", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Reading", exact: true })
  ).toBeVisible();
  await expect(
    page.getByRole("radiogroup", { name: "Default article view", exact: true })
  ).toBeVisible();
  await page.evaluate(() =>
    (
      window as Window & { xagaCapture?: { reset: () => void } }
    ).xagaCapture!.reset()
  );
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await expect(timeout).toHaveValue("10");
});

test("Settings retains refused numeric edits for retry", async ({ page }) => {
  await page.goto("/?window=settings&scenario=settings-save-failure");
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  const timeout = page.getByRole("textbox", {
    name: "Feed request timeout",
    exact: true,
  });
  await expect(timeout).toHaveValue("10");
  await timeout.fill("35");
  await timeout.press("Enter");
  await expect(page.getByRole("alert")).toContainText("Please try again");
  await expect(timeout).toHaveValue("35");
  await expect(timeout).toBeEnabled();
  await timeout.press("Enter");
  await expect(timeout).toBeEnabled();
  await page.getByRole("button", { name: "Privacy", exact: true }).click();
  await page.getByRole("button", { name: "General", exact: true }).click();
  await expect(timeout).toHaveValue("35");
});

test("Settings fields follow the library entry layout", async ({
  page,
}, testInfo) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await page.setViewportSize({ width: 900, height: 1000 });
  await page.goto("/?window=settings&theme=dark");
  const field = page.getByRole("textbox", {
    name: "Maximum feed response size",
    exact: true,
  });
  await expect(field).toBeEnabled();
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  const label = page
    .locator("label")
    .filter({ hasText: "Maximum feed response size" });
  for (const name of [
    "Show PDFs in the reading pane",
    "Refresh all feeds on start-up",
  ]) {
    const control = page.getByRole("checkbox", { name, exact: true });
    const labelId = await control.getAttribute("aria-labelledby");
    const helpId = await control.getAttribute("aria-describedby");
    const labelBox = (await page.locator(`[id="${labelId}"]`).boundingBox())!;
    const helpBox = (await page.locator(`[id="${helpId}"]`).boundingBox())!;
    expect(Math.abs(labelBox.x - helpBox.x)).toBeLessThan(1);
  }
  const labelBounds = (await label.boundingBox())!;
  const fieldBounds = (await field.boundingBox())!;
  expect(labelBounds.y + labelBounds.height).toBeLessThanOrEqual(fieldBounds.y);
  await page.screenshot({
    path: testInfo.outputPath("general-settings-dark.png"),
    fullPage: true,
  });
  await testInfo.attach("General settings", {
    path: testInfo.outputPath("general-settings-dark.png"),
    contentType: "image/png",
  });
});
