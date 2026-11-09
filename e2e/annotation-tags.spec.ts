import { test, expect, startCapture, openArticle, reader } from "./fixtures";

const garden = "A garden for every season";

test("assigns Annotation tags, browses the passage, and retains tags through recoloring", async ({
  page,
}) => {
  await startCapture(page);
  await openArticle(page, garden);
  await reader(page).locator("article p").first().click({ clickCount: 3 });
  await page.getByRole("button", { name: "violet", exact: true }).click();
  await page.getByRole("button", { name: "Show annotation details" }).click();
  const drawer = page.getByRole("dialog", { name: "Annotation details" });
  await drawer.getByRole("tab", { name: "Tags", exact: true }).click();
  const tags = drawer.getByRole("group", { name: "Annotation tags" });
  const projects = tags.getByRole("button", { name: "Projects", exact: true });
  await projects.focus();
  await page.keyboard.press("Enter");
  await expect(projects).toHaveAttribute("aria-pressed", "true");
  await drawer.getByPlaceholder("Find or create a tag").fill("Passages");
  await drawer.getByRole("button", { name: "Create “Passages”" }).click();
  await expect(
    tags.getByRole("button", { name: "Passages", exact: true })
  ).toHaveAttribute("aria-pressed", "true");
  await drawer
    .getByRole("button", { name: "Close details", exact: true })
    .click();
  await page.getByRole("button", { name: "red", exact: true }).click();
  await page.getByRole("button", { name: "Show annotation details" }).click();
  await expect(
    drawer.getByRole("button", { name: "Passages", exact: true })
  ).toHaveAttribute("aria-pressed", "true");
  await drawer
    .getByRole("button", { name: "Close details", exact: true })
    .click();
  await page.getByRole("button", { name: /^Tags \(/ }).click();
  await page.getByRole("checkbox", { name: "Passages", exact: true }).click();
  const passage = page.getByRole("button", { name: /^Small changes/ });
  await expect(passage).toBeVisible();
  await passage.click();
  await expect(passage).toHaveClass(/Mui-selected/);
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
  await expect(
    page.getByRole("checkbox", { name: "Passages", exact: true })
  ).toBeChecked();
});

test("shows annotation assignments read-only in the narrow drawer", async ({
  page,
}) => {
  await page.setViewportSize({ width: 700, height: 900 });
  await page.goto("/?scenario=read-only");
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await page.getByRole("button", { name: "All", exact: true }).click();
  await openArticle(page, "Making room for a workshop");
  await reader(page).locator("mark").first().click();
  const opener = page.getByRole("button", { name: "Show annotation details" });
  await opener.click();
  const drawer = page.getByRole("dialog", { name: "Annotation details" });
  await drawer.getByRole("tab", { name: "Tags", exact: true }).click();
  const projects = drawer
    .getByRole("group", { name: "Annotation tags" })
    .getByRole("button", { name: "Projects", exact: true });
  await expect(projects).toHaveAttribute("aria-pressed", "true");
  await expect(projects).toBeDisabled();
  await drawer.getByPlaceholder("Find or create a tag").fill("Read only");
  await expect(
    drawer.getByRole("button", { name: "Create “Read only”" })
  ).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(opener).toBeFocused();
});
