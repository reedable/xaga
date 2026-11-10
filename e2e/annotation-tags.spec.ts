import { control } from "./fixtures";
import { test, expect, startCapture, openArticle, reader } from "./fixtures";

const garden = "A garden for every season";

test("assigns Annotation tags, browses the passage, and retains tags through recoloring", async ({
  page,
}) => {
  await startCapture(page);
  await openArticle(page, garden);
  await reader(page).locator("article p").first().click({ clickCount: 3 });
  await control(page, "button", "violet").click();
  await control(page, "button", "annotation-details").click();
  const drawer = page.getByRole("dialog");
  await control(drawer, "tab", "inspector-tags").click();
  const tags = control(drawer, "group", "annotation-tags");
  const projects = tags.getByRole("button", { name: "Projects", exact: true });
  await projects.focus();
  await page.keyboard.press("Enter");
  await expect(projects).toHaveAttribute("aria-pressed", "true");
  await control(drawer, "textbox", "tag-search").fill("Passages");
  await control(drawer, "button", "create-tag").click();
  await expect(
    tags.getByRole("button", { name: "Passages", exact: true })
  ).toHaveAttribute("aria-pressed", "true");
  await control(drawer, "button", "close-details").click();
  await control(page, "button", "red").click();
  await control(page, "button", "annotation-details").click();
  await expect(
    drawer.getByRole("button", { name: "Passages", exact: true })
  ).toHaveAttribute("aria-pressed", "true");
  await control(drawer, "button", "close-details").click();
  await control(page, "button", "sidebar-tags-toggle").click();
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
  await control(page, "button", "filter-all").click();
  await openArticle(page, "Making room for a workshop");
  await reader(page).locator("mark").first().click();
  const opener = control(page, "button", "annotation-details");
  await opener.click();
  const drawer = page.getByRole("dialog");
  await control(drawer, "tab", "inspector-tags").click();
  const projects = control(drawer, "group", "annotation-tags").getByRole(
    "button",
    { name: "Projects", exact: true }
  );
  await expect(projects).toHaveAttribute("aria-pressed", "true");
  await expect(projects).toBeDisabled();
  await control(drawer, "textbox", "tag-search").fill("Read only");
  await expect(control(drawer, "button", "create-tag")).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(opener).toBeFocused();
});
