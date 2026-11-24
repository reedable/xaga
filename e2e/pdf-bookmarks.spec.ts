import type { Page } from "@playwright/test";
import {
  articleButton,
  control,
  expect,
  setSidebarVisible,
  test,
} from "./fixtures";

const title = "Field notes PDF";
const url = "https://fieldnotes.example/field-notes.pdf";

async function start(page: Page, scenario: string) {
  await page.goto(`/?scenario=${scenario}`);
  if (scenario === "pdf-bookmarks-read-only")
    await control(
      page.getByRole("dialog"),
      "button",
      "continue-read-only"
    ).click();
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
}

async function openPdf(page: Page, source = url) {
  const address = page.locator('input[name="article-url"]');
  await address.fill(source);
  await address.press("Enter");
}

test("first PDF bookmark reaches Saved and removal preserves the viewer and list", async ({
  page,
}) => {
  await start(page, "pdf-bookmarks");
  await openPdf(page);
  const viewer = page.getByTestId("pdf-reader");
  await expect(viewer).toBeVisible();
  await viewer.evaluate((frame) =>
    frame.setAttribute("data-test-viewer", "original")
  );

  const toggle = control(page, "button", "bookmark-toggle");
  const palette = control(page, "dialog", "bookmark-palette");
  const blue = control(palette, "button", "blue");
  await toggle.click();
  await blue.click(); // This fixture rejects the first write.
  await expect(blue).toHaveAttribute("aria-pressed", "false");
  await blue.click();
  await expect(palette).toHaveCount(0);
  await expect(viewer).toHaveAttribute("data-test-viewer", "original");

  const saved = control(page, "button", "filter-saved");
  await saved.click();
  const row = articleButton(page, title);
  await expect(row).toHaveAttribute("data-bookmark-color", "blue");
  await row.click();
  await expect(viewer).toBeVisible();

  await toggle.click();
  await expect(blue).toHaveAttribute("aria-pressed", "true");
  await control(palette, "button", "red").click();
  await expect(row).toHaveAttribute("data-bookmark-color", "red");

  const remove = async () => {
    await toggle.click();
    await control(palette, "button", "remove-marking").click();
    await control(palette, "button", "confirm-remove").click();
    await expect(palette).toHaveCount(0);
  };
  await remove(); // The Entry saves, but the fixture reports index failure.
  await expect(row).toHaveAttribute("data-bookmark-color", "");
  await expect(viewer).toHaveAttribute("data-test-viewer", "original");
  await remove(); // Retrying an already committed removal is still possible.

  const all = control(page, "button", "filter-all");
  await all.click();
  await expect(all).toHaveAttribute("aria-pressed", "true");
  await saved.click();
  await expect(row).toHaveCount(0);
});

test("external PDF presentation retains the bookmark controls", async ({
  page,
}) => {
  await start(page, "pdf-bookmarks-external");
  await openPdf(page);
  await control(page, "button", "bookmark-toggle").click();
  await control(
    control(page, "dialog", "bookmark-palette"),
    "button",
    "blue"
  ).click();
  await control(page, "button", "filter-saved").click();
  await expect(articleButton(page, title)).toHaveAttribute(
    "data-bookmark-color",
    "blue"
  );
  await expect(page.getByTestId("pdf-reader")).toHaveCount(0);
});

test("read-only and legacy PDFs remain viewable with marking unavailable", async ({
  page,
}) => {
  await start(page, "pdf-bookmarks-read-only");
  const toggle = control(page, "button", "bookmark-toggle");
  await control(page, "button", "filter-saved").click();
  await articleButton(page, title).click();
  await expect(page.getByTestId("pdf-reader")).toBeVisible();
  await expect(toggle).toBeDisabled();

  await start(page, "pdf-bookmarks");
  await openPdf(page, "https://fieldnotes.example/legacy.pdf");
  await expect(page.getByTestId("pdf-reader")).toBeVisible();
  await expect(toggle).toBeDisabled();
});

test("tags a PDF and reopens it from Tags without replacing the current viewer on save", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await start(page, "pdf-bookmarks");
  await openPdf(page);
  const viewer = page.getByTestId("pdf-reader");
  await expect(viewer).toBeVisible();
  await viewer.evaluate((frame) => (frame.dataset.testViewer = "original"));
  await expect(control(page, "tab", "inspector-related")).toBeVisible();
  await control(page, "tab", "inspector-topics").click();
  const topic = page.getByRole("checkbox", {
    name: "Everyday discoveries",
    exact: true,
  });
  await topic.check();
  await expect(topic).toBeChecked();
  await control(page, "tab", "inspector-tags").click();
  const projects = page.getByRole("button", { name: "Projects", exact: true });
  await projects.click();
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(projects).toHaveAttribute("aria-pressed", "false");
  await projects.click();
  await expect(projects).toHaveAttribute("aria-pressed", "true");
  await control(page, "textbox", "tag-search").fill("Field research");
  await control(page, "button", "create-tag").click();
  const research = page.getByRole("button", {
    name: "Field research",
    exact: true,
  });
  await expect(research).toHaveAttribute("aria-pressed", "true");
  await expect(viewer).toHaveAttribute("data-test-viewer", "original");

  await openPdf(page, "https://fieldnotes.example/legacy.pdf");
  await expect(control(page, "tab", "inspector-tags")).toHaveCount(0);
  await control(page, "button", "sidebar-tags-toggle").click();
  await page
    .getByRole("checkbox", { name: "Field research", exact: true })
    .click();
  const result = articleButton(page, title);
  await result.click();
  await expect(viewer).toBeVisible();
  await expect(page.locator('input[name="article-url"]')).toHaveValue(url);
  await research.click();
  await expect(research).toHaveAttribute("aria-pressed", "false");
  await expect(result).toBeVisible();
});

test("assigns and removes PDF tags in the narrow Inspector", async ({
  page,
}) => {
  await page.setViewportSize({ width: 700, height: 900 });
  await start(page, "pdf-bookmarks-external");
  await setSidebarVisible(page, false);
  await openPdf(page);
  const opener = control(page, "button", "article-details");
  await opener.click();
  const drawer = page.getByRole("dialog");
  await control(drawer, "tab", "inspector-tags").click();
  const projects = drawer.getByRole("button", {
    name: "Projects",
    exact: true,
  });
  await projects.click();
  await expect(projects).toHaveAttribute("aria-pressed", "true");
  await projects.click();
  await expect(projects).toHaveAttribute("aria-pressed", "false");
  await page.keyboard.press("Escape");
  await expect(opener).toBeFocused();
});

test("shows PDF tags in a read-only library", async ({ page }) => {
  await page.setViewportSize({ width: 700, height: 900 });
  await start(page, "pdf-bookmarks-read-only");
  await setSidebarVisible(page, false);
  await openPdf(page);
  await control(page, "button", "article-details").click();
  const drawer = page.getByRole("dialog");
  await control(drawer, "tab", "inspector-tags").click();
  const projects = drawer.getByRole("button", {
    name: "Projects",
    exact: true,
  });
  await expect(projects).toHaveAttribute("aria-pressed", "true");
  await expect(projects).toBeDisabled();
  await control(drawer, "textbox", "tag-search").fill("New tag");
  await expect(control(drawer, "button", "create-tag")).toBeDisabled();
});
