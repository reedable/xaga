import { control } from "./fixtures";
import { test, expect } from "./fixtures";

test("Settings exposes fetch controls, preserves exact values, and retains panel organization", async ({
  page,
}) => {
  await page.goto("/?window=settings");
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await expect(control(page, "button", "switch-library")).toBeVisible();
  await expect(
    control(page, "checkbox", "inline-pdf-reader")
  ).toHaveAccessibleDescription(/\S/);
  await expect(
    control(page, "checkbox", "refresh-on-startup")
  ).toHaveAccessibleDescription(/\S/);
  const timeout = control(page, "textbox", "loaderTimeoutSecs");
  await expect(timeout).toHaveValue("10");
  await timeout.fill("9007199254740993");
  await timeout.press("Enter");
  await expect(timeout).toBeEnabled();
  await control(page, "button", "settings-security").click();
  const http = control(page, "switch", "httpAllowed");
  await expect(http).not.toBeChecked();
  await http.check();
  await expect(http).toBeChecked();
  await expect(
    control(page, "switch", "connectNonstandardPortsAllowed")
  ).not.toBeChecked();
  const redirects = control(page, "textbox", "maxRedirects");
  await expect(redirects).toHaveValue("3");
  await redirects.fill("11");
  await redirects.press("Enter");
  await expect(redirects).toHaveAttribute("aria-invalid", "true");
  await redirects.fill("0");
  await redirects.press("Enter");
  await expect(redirects).toHaveAttribute("aria-invalid", "false");
  await control(page, "button", "settings-privacy").click();
  await expect(page.getByTestId("website-sessions")).toBeVisible();
  await expect(control(page, "switch", "refererAllowed")).not.toBeChecked();
  await control(page, "switch", "gzipAllowed").check();
  await control(page, "button", "settings-general").click();
  await expect(timeout).toHaveValue("9007199254740993");
  await expect(control(page, "textbox", "feedMaxBodyBytes")).toHaveValue("10");
  const feedLimit = control(page, "textbox", "feedMaxBodyBytes");
  await feedLimit.fill("12.5");
  await feedLimit.press("Enter");
  await control(page, "button", "settings-security").click();
  await control(page, "button", "settings-general").click();
  await expect(feedLimit).toHaveValue("12.5");
  await control(page, "button", "settings-appearance").click();
  await expect(control(page, "heading", "reading-settings")).toBeVisible();
  await expect(control(page, "radiogroup", "article-view")).toBeVisible();
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
  const timeout = control(page, "textbox", "feedTimeoutSecs");
  await expect(timeout).toHaveValue("10");
  await timeout.fill("35");
  await timeout.press("Enter");
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(timeout).toHaveValue("35");
  await expect(timeout).toBeEnabled();
  await timeout.press("Enter");
  await expect(timeout).toBeEnabled();
  await control(page, "button", "settings-privacy").click();
  await control(page, "button", "settings-general").click();
  await expect(timeout).toHaveValue("35");
});

test("Settings fields follow the library entry layout", async ({
  page,
}, testInfo) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await page.setViewportSize({ width: 900, height: 1000 });
  await page.goto("/?window=settings&theme=dark");
  const field = control(page, "textbox", "feedMaxBodyBytes");
  await expect(field).toBeEnabled();
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  const fieldId = await field.getAttribute("id");
  const label = page.locator(`label[for="${fieldId}"]`);
  for (const name of ["inline-pdf-reader", "refresh-on-startup"]) {
    const fieldControl = control(page, "checkbox", name);
    const labelId = await fieldControl.getAttribute("aria-labelledby");
    const helpId = await fieldControl.getAttribute("aria-describedby");
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
