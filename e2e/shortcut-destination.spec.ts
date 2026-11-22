import type { Page } from "@playwright/test";
import {
  control,
  expect,
  openArticle,
  reader,
  shareAction,
  test,
} from "./fixtures";

const additionalFeedUrl = "https://workshop.example/feed.xml";
const garden = "A garden for every season";
const destination = (page: Page) =>
  control(page, "combobox", "shortcut-location");
const group = (page: Page, id: string) =>
  page.locator(`[data-shortcut-group="${id}"]`);
async function start(page: Page, scenario = "shortcut-destination") {
  await page.goto(`/?scenario=${scenario}`);
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
}
const inGroup = (page: Page, id: string, title: string) =>
  group(page, id).getByRole("button", {
    name: title,
    exact: true,
    includeHidden: true,
  });

test("Article Share offers distinct nested paths and allows keyboard cancellation", async ({
  page,
}) => {
  await start(page);
  await openArticle(page, garden);
  await shareAction(page, "pin-toggle");
  const select = destination(page);
  await expect(select).toHaveAccessibleName(/\S/);
  await expect(select).toHaveValue("root");
  await expect(select).toBeFocused();
  const paths = await select
    .locator('option[value^="group:"]')
    .allTextContents();
  expect(new Set(paths).size).toBe(paths.length);
  await select.selectOption("group:notes-two");
  await page.keyboard.press("Escape");
  await expect(select).toHaveCount(0);
  await expect(control(page, "button", "share")).toBeFocused();
  await expect(inGroup(page, "notes-two", garden)).toHaveCount(0);
  await shareAction(page, "pin-toggle");
  await select.selectOption("group:notes-two");
  const add = control(page.getByRole("dialog"), "button", "add");
  await add.focus();
  await page.keyboard.press("Enter");
  await expect(inGroup(page, "notes-two", garden)).toHaveCount(1);
  await expect(inGroup(page, "notes-one", garden)).toHaveCount(0);
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    garden
  );
});

test("a Feed shortcut can be retried in a different folder and moved on Edit", async ({
  page,
}) => {
  await start(page, "shortcut-destination-retry");
  await control(page, "button", "add-feed").click();
  const dialog = page
    .getByRole("dialog")
    .filter({ has: page.getByTestId("shortcut-location") });
  const title = "Workshop notes";
  await control(dialog, "textbox", "feed-url").fill(additionalFeedUrl);
  await control(dialog, "textbox", "collection-name").fill(title);
  await destination(page).selectOption("group:notes-one");
  await control(dialog, "button", "add").click();
  await expect(control(dialog, "textbox", "collection-name")).toBeDisabled();
  await expect(destination(page)).toHaveValue("group:notes-one");
  await destination(page).selectOption("group:notes-two");
  await control(dialog, "button", "add").click();
  await expect(dialog).toBeHidden();
  await expect(inGroup(page, "notes-two", title)).toHaveCount(1);
  await control(page, "button", "edit-collection").click();
  await expect(destination(page)).toHaveValue("group:notes-two");
  await destination(page).selectOption("group:personal");
  await control(dialog, "button", "save").click();
  await expect(inGroup(page, "notes-two", title)).toHaveCount(0);
  await expect(inGroup(page, "personal", title)).toHaveCount(1);
});

test("a Topic form adds a shortcut directly into a nested group", async ({
  page,
}) => {
  await start(page);
  await control(page, "button", "add-topic").click();
  const title = "Seasonal sketches";
  await control(page, "textbox", "topic-title").fill(title);
  await destination(page).selectOption("group:notes-one");
  await control(page.getByRole("dialog"), "button", "save").click();
  await expect(inGroup(page, "notes-one", title)).toHaveCount(1);
});

test("Site Settings shows the current location and moves a shortcut without navigation", async ({
  page,
}) => {
  await start(page);
  await control(page, "button", "sidebar-sites-toggle").click();
  const host = "fieldnotes.example";
  await control(page, "list", "sidebar-sites-list")
    .getByRole("button", { name: host, exact: false })
    .click();
  const edit = control(page, "button", "edit-collection");
  await edit.click();
  const dialog = page.getByRole("dialog");
  await expect(destination(page)).toHaveValue("none");
  await destination(page).selectOption("group:notes-one");
  await control(dialog, "button", "save").click();
  await expect(inGroup(page, "notes-one", host)).toHaveCount(1);
  await edit.click();
  await expect(destination(page)).toHaveValue("group:notes-one");
  await destination(page).selectOption("group:personal");
  await control(dialog, "button", "save").click();
  await expect(inGroup(page, "notes-one", host)).toHaveCount(0);
  await expect(inGroup(page, "personal", host)).toHaveCount(1);
  await expect(edit).toBeVisible();
});
