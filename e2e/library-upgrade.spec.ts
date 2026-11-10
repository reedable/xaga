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
  const badge = page.getByRole("button", { name: "Read-only library options" });
  await expect(badge).toBeVisible();
  await badge.click();
}

for (const theme of ["light", "dark"] as const) {
  test(`library upgrade confirmation and toolbar appearance in ${theme}`, async ({
    page,
  }, testInfo) => {
    await openOptions(page, "upgrade-older", theme);
    const badge = page.getByRole("button", {
      name: "Read-only library options",
      includeHidden: true,
    });
    await expect(badge).toHaveAttribute("aria-expanded", "true");
    await expect(page.locator(".MuiMenu-paper")).toHaveCSS("opacity", "1");
    const sort = page.getByRole("button", {
      name: "Sort collections",
      includeHidden: true,
    });
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
        path: testInfo.outputPath(`upgrade-${theme}-badge.png`),
      }),
      contentType: "image/png",
    });
    await page
      .getByRole("menuitem", { name: "Upgrade Library", exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await expect(page.locator(".MuiDialog-container")).toHaveCSS(
      "opacity",
      "1"
    );
    await expect(dialog).toContainText("back up the complete library");
    await expect(dialog).toContainText("cannot be undone");
    await expect(dialog).toContainText("close all other processes");
    await testInfo.attach(`confirmation-${theme}`, {
      body: await page.screenshot({
        animations: "disabled",
        path: testInfo.outputPath(`upgrade-${theme}-confirmation.png`),
      }),
      contentType: "image/png",
    });
    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(
      page
        .locator("[data-library-toolbar]")
        .getByRole("button", { name: "Refresh", exact: true })
    ).toBeDisabled();
  });
}

test("upgrade progress, failure, retry and restart failure keep writes disabled", async ({
  page,
}) => {
  await openOptions(page, "upgrade-failure");
  await page
    .getByRole("menuitem", { name: "Upgrade Library", exact: true })
    .click();
  const dialog = page.getByRole("dialog");
  await dialog
    .getByRole("button", { name: "Upgrade Library", exact: true })
    .click();
  await expect(dialog.getByRole("status")).toContainText("Converting library");
  await expect(
    dialog.getByRole("button", { name: "Upgrade Library", exact: true })
  ).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("alert")).toContainText(
    "Library upgrade incomplete"
  );
  await dialog.getByRole("button", { name: "Retry Library Upgrade" }).click();
  await expect(dialog.getByRole("heading")).toHaveText(
    "Library upgrade complete"
  );
  await expect(dialog.getByRole("alert")).toContainText("could not restart");
  await expect(
    dialog.getByRole("button", { name: "Restart Xaga" })
  ).toBeEnabled();
  await dialog.getByRole("button", { name: "Close", exact: true }).click();
  await expect(
    page
      .locator("[data-library-toolbar]")
      .getByRole("button", { name: "Refresh", exact: true })
  ).toBeDisabled();
});

for (const scenario of ["unversioned", "resume"]) {
  test(`${scenario} library offers explicit confirmation`, async ({ page }) => {
    await openOptions(page, `upgrade-${scenario}`);
    const action =
      scenario === "resume" ? "Resume Library Upgrade" : "Upgrade Library";
    await page.getByRole("menuitem", { name: action, exact: true }).click();
    const dialog = page.getByRole("dialog");
    if (scenario === "resume")
      await expect(dialog).toContainText("already partially applied");
    else await expect(dialog).toContainText("no version metadata");
    await expect(
      dialog.getByRole("button", { name: action, exact: true })
    ).toBeEnabled();
  });
}

test("newer library offers Upgrade Xaga and reports browser-opening refusal", async ({
  page,
}) => {
  await openOptions(page, "upgrade-newer");
  await expect(
    page.getByRole("menuitem", { name: "Upgrade Library", exact: true })
  ).toHaveCount(0);
  await page
    .getByRole("menuitem", { name: "Upgrade Xaga", exact: true })
    .click();
  await expect(page.getByRole("dialog").getByRole("alert")).toContainText(
    "releases page could not be opened"
  );
});

for (const [scenario, text] of [
  ["upgrade-pending-other", "Resume it using Xaga 0.17"],
  ["upgrade-invalid", "metadata is malformed"],
  ["read-only", "No upgrade is required"],
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
      page.getByRole("button", { name: "Read-only library options" })
    ).toHaveCount(0);
    const badge = page.getByText("Read-only", { exact: true });
    await badge.focus();
    await expect(page.getByRole("tooltip")).toContainText(text);
  });
}

test("upgrade progress survives the sidebar being hidden by a narrow window", async ({
  page,
}) => {
  await openOptions(page, "upgrade-older");
  await page
    .getByRole("menuitem", { name: "Upgrade Library", exact: true })
    .click();
  const dialog = page.getByRole("dialog");
  await dialog
    .getByRole("button", { name: "Upgrade Library", exact: true })
    .click();
  await page.setViewportSize({ width: 700, height: 900 });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("heading")).toHaveText(
    "Library upgrade complete"
  );
  await expect(dialog.getByRole("alert")).toContainText("could not restart");
});
