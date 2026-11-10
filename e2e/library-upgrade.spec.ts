import { control, test, expect, openArticle } from "./fixtures";

async function openChoice(
  page: import("@playwright/test").Page,
  scenario: string
) {
  await page.goto(`/?scenario=${scenario}`);
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAccessibleName(/\S/);
  await expect(dialog).toHaveAccessibleDescription(/\S/);
  // The choice is reachable before normal startup or the sidebar exists.
  await expect(page.locator("html")).not.toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  return dialog;
}

for (const theme of ["light", "dark"] as const) {
  test(`automatic confirmation and reopened badge in ${theme}`, async ({
    page,
  }, testInfo) => {
    await page.emulateMedia({ colorScheme: theme });
    const dialog = await openChoice(page, `upgrade-older&theme=${theme}`);
    await expect(dialog).toHaveAttribute("data-library-version", "0.15");
    await expect(dialog).toHaveAttribute("data-target-version", "0.16");
    await testInfo.attach(`confirmation-${theme}`, {
      body: await page.screenshot({
        animations: "disabled",
        path: testInfo.outputPath(
          `upgrade-${theme}-${(await dialog.isVisible()) ? "confirmation" : "badge"}.png`
        ),
      }),
      contentType: "image/png",
    });
    await control(dialog, "button", "continue-read-only").click();
    await expect(dialog).not.toBeVisible();
    await expect(page.locator("html")).toHaveAttribute(
      "data-capture-ready",
      "true"
    );
    const badge = control(page, "button", "library-options-trigger");
    await expect(badge).toHaveAccessibleName(/\S/);
    await expect(badge).toHaveAttribute("aria-haspopup", "dialog");
    await badge.focus();
    await expect(page.getByRole("tooltip")).toHaveCount(0);
    const sort = control(page, "button", "sidebar-sort-trigger");
    const badgeBox = (await badge.boundingBox())!;
    const sortBox = (await sort.boundingBox())!;
    expect(badgeBox.x + badgeBox.width).toBeLessThanOrEqual(sortBox.x);
    expect(
      Math.abs(
        badgeBox.y + badgeBox.height / 2 - sortBox.y - sortBox.height / 2
      )
    ).toBeLessThan(2);
    await testInfo.attach(`badge-${theme}`, {
      body: await page.screenshot({
        animations: "disabled",
        path: testInfo.outputPath(
          `upgrade-${theme}-${(await dialog.isVisible()) ? "confirmation" : "badge"}.png`
        ),
      }),
      contentType: "image/png",
    });
    // A normal sidebar rerender must not reopen the choice.
    await sort.click();
    await page.keyboard.press("Escape");
    await expect(dialog).not.toBeVisible();
    await badge.click();
    await expect(dialog).toBeVisible();
    await control(dialog, "button", "continue-read-only").click();
    await expect(
      control(page.locator("[data-library-toolbar]"), "button", "refresh")
    ).toBeDisabled();
  });
}

test("failure retry and restart failure preserve read-only state", async ({
  page,
}) => {
  const dialog = await openChoice(page, "upgrade-failure");
  const confirmation = await dialog
    .locator("#library-upgrade-description")
    .textContent();
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
  await expect(dialog.locator("#library-upgrade-description")).not.toHaveText(
    confirmation!
  );
  await expect(dialog.getByRole("status")).toHaveText(/\S/);
  await expect(control(dialog, "button", "restart-xaga")).toBeEnabled();
  await control(dialog, "button", "restart-xaga").click();
  await expect(control(dialog, "button", "restart-xaga")).toBeEnabled();
  await control(dialog, "button", "continue-read-only").click();
  await expect(
    control(page.locator("[data-library-toolbar]"), "button", "refresh")
  ).toBeDisabled();
});

for (const scenario of ["unversioned", "resume", "resume-earlier"]) {
  test(`${scenario} offers explicit upgrade`, async ({ page }) => {
    const dialog = await openChoice(page, `upgrade-${scenario}`);
    await expect(dialog).toHaveAttribute(
      "data-read-only-cause",
      scenario.startsWith("resume") ? "pending_upgrade" : "unversioned"
    );
    await expect(control(dialog, "button", "upgrade-library")).toBeEnabled();
  });
}

for (const scenario of ["newer", "pending-other"]) {
  test(`${scenario} offers app update without conversion`, async ({ page }) => {
    const dialog = await openChoice(page, `upgrade-${scenario}`);
    await expect(control(dialog, "button", "upgrade-library")).toHaveCount(0);
    await control(dialog, "button", "upgrade-xaga").click();
    await expect(dialog.getByRole("alert")).toBeVisible();
    await expect(control(dialog, "button", "upgrade-xaga")).toBeEnabled();
  });
}

for (const [scenario, cause] of [
  ["upgrade-invalid", "invalid_metadata"],
  ["read-only", "lock_unavailable"],
]) {
  test(`${scenario} explains without unsafe action`, async ({ page }) => {
    const dialog = await openChoice(page, scenario);
    await expect(dialog).toHaveAttribute("data-read-only-cause", cause);
    await expect(dialog.getByRole("button")).toHaveCount(1);
    await control(dialog, "button", "continue-read-only").click();
    await control(page, "button", "library-options-trigger").click();
    await expect(dialog).toBeVisible();
  });
}

test("progress survives a hidden sidebar", async ({ page }) => {
  const dialog = await openChoice(page, "upgrade-older");
  await control(dialog, "button", "upgrade-library").click();
  await page.setViewportSize({ width: 700, height: 900 });
  await expect(dialog).toHaveAttribute("data-upgrade-state", "succeeded");
  await expect(dialog.getByRole("alert")).toBeVisible();
});

test("transient polling failure retries observation and does not stop conversion", async ({
  page,
}) => {
  const dialog = await openChoice(page, "upgrade-poll-failure");
  await control(dialog, "button", "upgrade-library").click();
  await expect(dialog.getByRole("alert")).toBeVisible();
  await expect(dialog).toHaveAttribute("data-upgrade-state", "running");
  await expect(dialog).toHaveAttribute("data-upgrade-state", "succeeded");
  await expect(control(dialog, "button", "restart-xaga")).toBeEnabled();
});

for (const scenario of ["upgrade-model-delayed", "upgrade-model-unavailable"]) {
  test(`${scenario}: upgrading bypasses model initialization`, async ({
    page,
  }) => {
    const dialog = await openChoice(page, scenario);
    await control(dialog, "button", "upgrade-library").click();
    await expect(dialog).toHaveAttribute("data-upgrade-state", "succeeded");
    await expect(page.locator("html")).not.toHaveAttribute(
      "data-capture-ready",
      "true"
    );
  });
  test(`${scenario}: continuing read-only runs normal initialization`, async ({
    page,
  }) => {
    const dialog = await openChoice(page, scenario);
    await control(dialog, "button", "continue-read-only").click();
    await expect(page.locator("html")).toHaveAttribute(
      "data-capture-ready",
      "true"
    );
    await expect(dialog).not.toBeVisible();
    await expect(
      control(page, "button", "library-options-trigger")
    ).toBeVisible();
  });
}

test("failed recovery hides unknown counts and Topic checkboxes", async ({
  page,
}) => {
  const dialog = await openChoice(page, "upgrade-read-failure");
  await control(dialog, "button", "upgrade-library").click();
  await expect(dialog).toHaveAttribute("data-upgrade-state", "failed");
  await expect(dialog.getByRole("alert")).toHaveCount(2);
  await control(dialog, "button", "continue-read-only").click();
  // Capture readiness requires counts; this fault intentionally withholds them.
  await expect(control(page, "button", "all-items")).toBeVisible();
  await expect(
    control(page, "button", "all-items").locator("..")
  ).not.toContainText(/\d/);
  await openArticle(page, "A garden for every season");
  await control(page, "tab", "inspector-topics").click();
  await expect(
    control(page, "button", "retry-topic-memberships")
  ).toBeVisible();
  await expect(page.getByRole("checkbox")).toHaveCount(0);
  await control(page, "button", "retry-topic-memberships").click();
  await expect(
    control(page, "button", "retry-topic-memberships")
  ).toBeVisible();
});

test("reopening checks status again after an external manifest change", async ({
  page,
}) => {
  const dialog = await openChoice(page, "upgrade-status-change");
  await expect(control(dialog, "button", "upgrade-library")).toBeVisible();
  const description = await dialog
    .locator("#library-upgrade-description")
    .textContent();
  await control(dialog, "button", "continue-read-only").click();
  await control(page, "button", "library-options-trigger").click();
  await expect(dialog).toHaveAttribute(
    "data-read-only-cause",
    "pending_other_version"
  );
  await expect(control(dialog, "button", "upgrade-library")).toHaveCount(0);
  await expect(control(dialog, "button", "upgrade-xaga")).toBeVisible();
  await expect(dialog.locator("#library-upgrade-description")).not.toHaveText(
    description!
  );
});
