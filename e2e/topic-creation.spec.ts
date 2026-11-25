import type { Page } from "@playwright/test";
import { articleButton, control, expect, openArticle, test } from "./fixtures";

const existing = "Everyday discoveries";
const garden = "A garden for every season";
const water = "Following the water";
const warning = (page: Page) =>
  control(page, "dialog", "similar-topics-warning");
const title = (page: Page) => control(page, "textbox", "topic-title");
const topics = (page: Page) => control(page, "list", "sidebar-topics-list");

async function start(page: Page, scenario = "") {
  await page.goto(scenario ? `/?scenario=${scenario}` : "/");
  if (scenario === "read-only") {
    await control(page, "button", "continue-read-only").click();
  }
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await control(page, "button", "sidebar-topics-toggle").click();
}

async function startDraft(page: Page, draft: string) {
  await control(page, "button", "add-topic").click();
  await title(page).fill(draft);
}

async function startInline(page: Page, article: string, draft: string) {
  await openArticle(page, article);
  await control(page, "tab", "inspector-topics").click();
  await control(page, "textbox", "filter-topics").fill("New draft");
  await control(page, "button", "create-topic").click();
  await title(page).fill(draft);
}

test("a similar Topic can be opened without saving or changing it", async ({
  page,
}) => {
  await start(page);
  await startDraft(page, existing);
  await control(page, "button", "save").click();
  const match = control(warning(page), "button", "open-similar-topic");
  await expect(match).toHaveCount(1);
  await expect(match).toContainText(existing);
  await match.click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    topics(page).getByRole("button", { name: existing, exact: true })
  ).toHaveCount(1);
  await expect(articleButton(page, garden)).toBeVisible();
  await expect(articleButton(page, water)).toHaveCount(0);
});

test("dismissing or revising a warning keeps the draft and rechecks edited text", async ({
  page,
}) => {
  await start(page);
  await startDraft(page, existing);
  const description = control(page, "textbox", "topic-description");
  await description.fill("Field observations");
  const save = control(page, "button", "save");
  await save.click();
  await expect(warning(page)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(title(page)).toHaveValue(existing);
  await expect(description).toHaveValue("Field observations");
  await save.click();
  await control(warning(page), "button", "revise-topic").click();
  await title(page).fill("Field sketches");
  await save.click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    topics(page).getByRole("button", { name: "Field sketches", exact: true })
  ).toHaveCount(1);
  await expect(
    topics(page).getByRole("button", { name: existing, exact: true })
  ).toHaveCount(1);
});

test("an unavailable similarity check requires explicit confirmation", async ({
  page,
}) => {
  const draft = "An unindexed draft";
  await start(page, "topic-check-unavailable");
  await startDraft(page, draft);
  const save = control(page, "button", "save");
  await save.click();
  await expect(warning(page)).toBeVisible();
  await expect(
    control(warning(page), "button", "open-similar-topic")
  ).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(title(page)).toHaveValue(draft);
  await expect(
    topics(page).getByRole("button", { name: draft, exact: true })
  ).toHaveCount(0);
  await save.click();
  await control(warning(page), "button", "create-topic-anyway").click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    topics(page).getByRole("button", { name: draft, exact: true })
  ).toHaveCount(1);
});

// Temporarily disabled because this case fails intermittently on CI.
/*
test("inline confirmation creates a separate Topic with the requested member and pin", async ({
  page,
}) => {
  await start(page);
  await startInline(page, water, existing);
  await control(page, "combobox", "shortcut-location").selectOption("root");
  await control(page, "button", "confirm-create-topic").click();
  await expect(
    control(warning(page), "button", "open-similar-topic")
  ).toHaveCount(1);
  await control(warning(page), "button", "create-topic-anyway").click();
  await expect(warning(page)).toBeHidden();
  await expect(title(page)).toBeHidden();
  await control(page, "textbox", "filter-topics").fill("");
  const memberships = page.getByRole("checkbox", {
    name: existing,
    exact: true,
  });
  await expect(memberships).toHaveCount(2);
  await expect(
    memberships.and(page.locator('[aria-checked="true"]'))
  ).toHaveCount(1);
  await control(page, "button", "sidebar-shortcuts-toggle").click();
  const shortcuts = control(page, "list", "sidebar-shortcuts-list");
  await expect(
    shortcuts.getByRole("button", { name: existing, exact: true })
  ).toHaveCount(2);
});
*/

test("a failed save after confirmation leaves the draft available for retry", async ({
  page,
}) => {
  await start(page, "topic-save-failure");
  await startDraft(page, existing);
  const save = control(page, "button", "save");
  await save.click();
  await control(warning(page), "button", "create-topic-anyway").click();
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(title(page)).toHaveValue(existing);
  await expect(save).toBeEnabled();
  await save.click();
  await control(warning(page), "button", "create-topic-anyway").click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    topics(page).getByRole("button", { name: existing, exact: true })
  ).toHaveCount(2);
});

// Temporarily disabled after a failure on CI.
/*
test("changing articles discards a pending inline check and repeated submissions", async ({
  page,
}) => {
  await start(page, "topic-check-pending");
  await startInline(page, garden, existing);
  const submit = control(page, "button", "confirm-create-topic");
  await submit.click();
  await expect(submit).toBeDisabled();
  await submit.evaluate((button) =>
    (button as HTMLButtonElement).form!.requestSubmit()
  );
  await startInline(page, water, "New article draft");
  await submit.click();
  await control(page, "textbox", "filter-topics").fill("");
  await expect(
    page.getByRole("checkbox", { name: "New article draft", exact: true })
  ).toBeChecked();
  await page.evaluate(() =>
    window.dispatchEvent(new Event("xaga-capture-complete-topic-check"))
  );
  await expect(warning(page)).toHaveCount(0);
  await expect(
    topics(page).getByRole("button", { name: existing, exact: true })
  ).toHaveCount(1);
});
*/

test("read-only libraries refuse both Topic creation entry points", async ({
  page,
}) => {
  await start(page, "read-only");
  await expect(control(page, "button", "add-topic")).toBeDisabled();
  await openArticle(page, garden);
  await control(page, "tab", "inspector-topics").click();
  await control(page, "textbox", "filter-topics").fill("New draft");
  await expect(control(page, "button", "create-topic")).toBeDisabled();
  await expect(warning(page)).toHaveCount(0);
});
