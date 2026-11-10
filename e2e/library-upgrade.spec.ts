import { control } from "./fixtures";
import { test, expect } from "./fixtures";

async function openOptions(
  page: import("@playwright/test").Page,
  scenario: string,
  theme: "light" | "dark" = "light"
) {
  await page.emulateMedia({ colorScheme: theme });
  await page.goto(`/?scenario=${scenario}&theme=${theme}`);
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  const badge = control(page, "button", "library-options-trigger");
  await expect(badge).toBeVisible();
  await badge.click();
}

for (const theme of ["light", "dark"] as const) {
  test(`library upgrade confirmation and toolbar appearance in ${theme}`, async ({
    page,
  }, testInfo) => {
    await openOptions(page, "upgrade-older", theme);
    const badge = control(page, "button", "library-options-trigger", true);
    await expect(badge).toHaveAttribute("aria-expanded", "true");
    await expect(page.locator(".MuiMenu-paper")).toHaveCSS("opacity", "1");
    const sort = control(page, "button", "sidebar-sort-trigger", true);
    const badgeBox = (await badge.boundingBox())!;
    const sortBox = (await sort.boundingBox())!;
    const menuBox = (await page.locator(".MuiMenu-paper").boundingBox())!;
    expect(Math.abs(menuBox.x - badgeBox.x)).toBeLessThan(1);
    expect(Math.abs(menuBox.y - badgeBox.y - badgeBox.height)).toBeLessThan(1);
    expect(badgeBox.x + badgeBox.width).toBeLessThanOrEqual(sortBox.x);
    expect(
      Math.abs(
        badgeBox.y + badgeBox.height / 2 - sortBox.y - sortBox.height / 2
      )
    ).toBeLessThan(2);
    await testInfo.attach(`badge-${theme}`, {
      body: await page.screenshot({
        animations: "disabled",
        path: testInfo.outputPath(`upgrade-${theme}-badge.png`),
      }),
      contentType: "image/png",
    });
    await control(page, "menuitem", "upgrade-library").click();
    const dialog = page.getByRole("dialog");
    await expect(page.locator(".MuiDialog-container")).toHaveCSS(
      "opacity",
      "1"
    );
    await expect(dialog).toHaveAttribute("data-upgrade-state", "idle");
    await expect(dialog).toHaveAttribute("data-library-version", "0.15");
    await expect(dialog).toHaveAttribute("data-target-version", "0.16");
    await expect(dialog.getByRole("heading")).toHaveAccessibleName(/\S/);
    await testInfo.attach(`confirmation-${theme}`, {
      body: await page.screenshot({
        animations: "disabled",
        path: testInfo.outputPath(`upgrade-${theme}-confirmation.png`),
      }),
      contentType: "image/png",
    });
    await control(dialog, "button", "cancel").click();
    await expect(dialog).not.toBeVisible();
    await expect(
      control(page.locator("[data-library-toolbar]"), "button", "refresh")
    ).toBeDisabled();
  });
}

test("upgrade progress, failure, retry and restart failure keep writes disabled", async ({
  page,
}) => {
  await openOptions(page, "upgrade-failure");
  await control(page, "menuitem", "upgrade-library").click();
  const dialog = page.getByRole("dialog");
  await control(dialog, "button", "upgrade-library").click();
  await expect(dialog).toHaveAttribute("data-upgrade-state", "running");
  await expect(dialog.getByRole("status")).toBeVisible();
  await expect(control(dialog, "button", "upgrade-library")).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAttribute("data-upgrade-state", "failed");
  await expect(dialog.getByRole("alert")).toBeVisible();
  await control(dialog, "button", "retry-library-upgrade").click();
  await expect(dialog).toHaveAttribute("data-upgrade-state", "succeeded");
  await expect(dialog.getByRole("alert")).toBeVisible();
  await expect(control(dialog, "button", "restart-xaga")).toBeEnabled();
  await control(dialog, "button", "close").click();
  await expect(
    control(page.locator("[data-library-toolbar]"), "button", "refresh")
  ).toBeDisabled();
});

for (const scenario of ["unversioned", "resume"]) {
  test(`${scenario} library offers explicit confirmation`, async ({ page }) => {
    await openOptions(page, `upgrade-${scenario}`);
    const action = "upgrade-library";
    await control(page, "menuitem", action).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toHaveAttribute(
      "data-read-only-cause",
      scenario === "resume" ? "pending_upgrade" : "unversioned"
    );
    await expect(control(dialog, "button", action)).toBeEnabled();
  });
}

test("newer library offers Upgrade Xaga and reports browser-opening refusal", async ({
  page,
}) => {
  await openOptions(page, "upgrade-newer");
  await expect(control(page, "menuitem", "upgrade-library")).toHaveCount(0);
  await control(page, "menuitem", "upgrade-xaga").click();
  await expect(page.getByRole("dialog").getByRole("alert")).toBeVisible();
  await expect(
    control(page.getByRole("dialog"), "button", "upgrade-xaga")
  ).toBeEnabled();
});

for (const [scenario, cause] of [
  ["upgrade-pending-other", "pending_other_version"],
  ["upgrade-invalid", "invalid_metadata"],
  ["read-only", "lock_unavailable"],
]) {
  test(`${scenario} explains read-only status without upgrade actions`, async ({
    page,
  }) => {
    await page.goto(`/?scenario=${scenario}`);
    await expect(page.locator("html")).toHaveAttribute(
      "data-capture-ready",
      "true"
    );
    await expect(
      control(page, "button", "library-options-trigger")
    ).toHaveCount(0);
    const badge = page.getByTestId("library-read-only");
    await badge.focus();
    await expect(badge).toHaveAttribute("data-read-only-cause", cause);
    await expect(page.getByRole("tooltip")).toBeVisible();
  });
}

test("upgrade progress survives the sidebar being hidden by a narrow window", async ({
  page,
}) => {
  await openOptions(page, "upgrade-older");
  await control(page, "menuitem", "upgrade-library").click();
  const dialog = page.getByRole("dialog");
  await control(dialog, "button", "upgrade-library").click();
  await page.setViewportSize({ width: 700, height: 900 });
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAttribute("data-upgrade-state", "succeeded");
  await expect(dialog.getByRole("alert")).toBeVisible();
});
