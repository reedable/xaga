import type { Page } from "@playwright/test";
import { control, expect, test } from "./fixtures";

const heading = (page: Page, id: string) =>
  page.locator(`[data-group-heading="${id}"]`);
const groupButton = (page: Page, id: string) =>
  heading(page, id).locator(":scope > [aria-expanded]");
const node = (page: Page, id: string) =>
  page.locator(`[data-shortcut-group="${id}"]`);
async function start(
  page: Page,
  height = 900,
  theme: "light" | "dark" = "light",
  motion: "reduce" | "no-preference" = "reduce"
) {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: motion });
  await page.setViewportSize({ width: 1440, height });
  await page.goto(`/?scenario=sidebar-sticky&theme=${theme}`);
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await expect(heading(page, "sources")).toBeVisible();
}
async function scrollGroup(page: Page, id: string, distance: number) {
  await node(page, id).evaluate((element, distance) => {
    let scroll = element.parentElement!;
    while (getComputedStyle(scroll).overflowY !== "auto")
      scroll = scroll.parentElement!;
    scroll.scrollLeft = 0;
    scroll.scrollTop +=
      element.getBoundingClientRect().top -
      scroll.getBoundingClientRect().top +
      distance;
  }, distance);
}
async function geometry(page: Page, id: string) {
  return heading(page, id).evaluate((element) => {
    let scroll = element.parentElement!;
    while (getComputedStyle(scroll).overflowY !== "auto")
      scroll = scroll.parentElement!;
    const rect = element.getBoundingClientRect();
    const font = parseFloat(
      getComputedStyle(document.documentElement).fontSize
    );
    return {
      top: rect.top / font,
      bottom: rect.bottom / font,
      height: rect.height / font,
      scrollTop: scroll.getBoundingClientRect().top / font,
      viewportHeight: scroll.clientHeight / font,
    };
  });
}
async function stacked(page: Page, ids: string[]) {
  await expect
    .poll(async () => {
      const boxes = await Promise.all(ids.map((id) => geometry(page, id)));
      return boxes.every(
        (box, index) =>
          Math.abs(
            box.top - (index ? boxes[index - 1].bottom : box.scrollTop)
          ) < 0.0625
      );
    })
    .toBe(true);
}
async function action(page: Page, id: string, actionId: string) {
  await heading(page, id).hover();
  await control(heading(page, id), "button", "shortcut-actions").click();
  await control(page, "menuitem", actionId).click();
}

for (const [height, theme] of [
  [900, "light"],
  [480, "dark"],
] as const) {
  test(`sticky ancestors stack and release at sibling and root boundaries at ${height}`, async ({
    page,
  }, testInfo) => {
    await start(page, height, theme);
    await scrollGroup(page, "research", 300);
    await stacked(page, ["research", "background", "sources"]);
    expect((await geometry(page, "background")).height).toBe(
      (await geometry(page, "research")).height
    );
    const accessible = await groupButton(page, "research").ariaSnapshot();
    const count = heading(page, "research")
      .locator("[data-unread-count]")
      .locator("..");
    await heading(page, "research").hover();
    await expect(count).toHaveCSS("opacity", "0");
    expect(await groupButton(page, "research").ariaSnapshot()).toBe(accessible);
    const actions = control(
      heading(page, "research"),
      "button",
      "shortcut-actions"
    );
    await actions.click();
    await expect(page.getByRole("menu")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(actions).toBeFocused();
    await page.mouse.move(1000, 400);
    await groupButton(page, "research").focus();
    await page.keyboard.press("Tab");
    await expect(actions).toBeFocused();
    const image = testInfo.outputPath(`sticky-${height}.png`);
    await page.screenshot({ path: image });
    await testInfo.attach("sticky-groups", {
      path: image,
      contentType: "image/png",
    });
    await scrollGroup(page, "methods", 120);
    await stacked(page, ["research", "methods"]);
    expect((await geometry(page, "background")).bottom).toBeLessThan(
      (await geometry(page, "research")).scrollTop
    );
    await scrollGroup(page, "work", 120);
    await stacked(page, ["work"]);
    expect((await geometry(page, "research")).bottom).toBeLessThan(
      (await geometry(page, "work")).scrollTop
    );
  });
}

test("group labels stay on one line through renaming, font scaling, and column resizing", async ({
  page,
}) => {
  await start(page);
  await scrollGroup(page, "research", 300);
  const before = await geometry(page, "research");
  await action(page, "research", "rename-group");
  const title = "Research notes with a long heading spanning several lines";
  await control(page, "textbox", "group-title").fill(title);
  await control(page.getByRole("dialog"), "button", "save").click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await scrollGroup(page, "research", 300);
  await stacked(page, ["research", "background", "sources"]);
  expect((await geometry(page, "research")).height).toBe(before.height);
  const label = heading(page, "research").locator("span[title]");
  await expect(groupButton(page, "research")).toHaveAccessibleName(
    new RegExp(title)
  );
  const singleLine = async () => {
    const size = await label.evaluate((element) => ({
      height: element.getBoundingClientRect().height,
      lineHeight: parseFloat(getComputedStyle(element).lineHeight),
      clipped: element.scrollWidth > element.clientWidth,
    }));
    expect(size.height).toBeCloseTo(size.lineHeight, 1);
    expect(size.clipped).toBe(true);
  };
  await singleLine();
  const resize = page.getByTestId("sidebar-column-resize");
  await resize.focus();
  await page.keyboard.press("End");
  await singleLine();
  await page.keyboard.press("Home");
  await singleLine();
  await page.addStyleTag({ content: "html { font-size: 125%; }" });
  await singleLine();
  await stacked(page, ["research", "background", "sources"]);
  await page.setViewportSize({ width: 1440, height: 600 });
  await expect(
    control(page, "combobox", "sidebar-list-selector")
  ).toBeVisible();
  await scrollGroup(page, "research", 300);
  await stacked(page, ["research", "background", "sources"]);
});

for (const motion of ["reduce", "no-preference"] as const) {
  test(`collapse and temporary filtered expansion preserve sticky headings with ${motion} motion`, async ({
    page,
  }) => {
    await start(page, 900, "light", motion);
    await groupButton(page, "background").click();
    await expect(groupButton(page, "background")).toHaveAttribute(
      "aria-expanded",
      "false"
    );
    await control(page, "button", "sidebar-sort-trigger").click();
    await control(page, "textbox", "filter-collections").fill("Collection 10");
    await page.keyboard.press("Escape");
    await expect(groupButton(page, "background")).toHaveAttribute(
      "aria-expanded",
      "true"
    );
    await expect(groupButton(page, "background")).toHaveAttribute(
      "aria-disabled",
      "true"
    );
    await expect(
      control(page, "list", "sidebar-shortcuts-list").getByRole("button", {
        name: "Collection 10",
        exact: true,
      })
    ).toBeVisible();
    await control(page, "button", "sidebar-sort-trigger").click();
    await control(page, "button", "clear-collection-filter").click();
    await page.keyboard.press("Escape");
    await expect(groupButton(page, "background")).toHaveAttribute(
      "aria-expanded",
      "false"
    );
    await groupButton(page, "background").click();
    await expect(
      node(page, "background").locator(":scope > .MuiCollapse-root")
    ).toHaveClass(/MuiCollapse-entered/);
    await scrollGroup(page, "research", 300);
    await stacked(page, ["research", "background", "sources"]);
  });
}

test("outermost five headings stick while deeper headings scroll normally", async ({
  page,
}, testInfo) => {
  await start(page);
  await scrollGroup(page, "Deep-1", 600);
  const ids = Array.from({ length: 5 }, (_, index) => `Deep-${index + 1}`);
  await stacked(page, ids);
  const sixth = await geometry(page, "Deep-6");
  await node(page, "Deep-1").evaluate((element) => {
    let scroll = element.parentElement!;
    while (getComputedStyle(scroll).overflowY !== "auto")
      scroll = scroll.parentElement!;
    scroll.scrollTop += 50;
  });
  await expect
    .poll(async () => (await geometry(page, "Deep-6")).top)
    .toBeLessThan(sixth.top);
  await stacked(page, ids);
  await expect(heading(page, "Deep-6")).toHaveCSS("position", "relative");
  // Record, rather than invent an adaptive policy for, the agreed fixed-cap limit.
  await page.setViewportSize({ width: 1440, height: 240 });
  await scrollGroup(page, "Five-1", 170);
  const data = await Promise.all(
    Array.from({ length: 5 }, (_, index) => geometry(page, `Five-${index + 1}`))
  );
  await testInfo.attach("short-window-heading-space", {
    body: JSON.stringify(data, null, 2),
    contentType: "application/json",
  });
  const image = testInfo.outputPath("fixed-cap-short-window.png");
  await page.screenshot({ path: image });
  await testInfo.attach("fixed-cap-short-window", {
    path: image,
    contentType: "image/png",
  });
});

test("keyboard traversal keeps focused shortcut controls visible beneath sticky ancestors when space permits", async ({
  page,
}) => {
  await start(page);
  await scrollGroup(page, "research", 300);
  await groupButton(page, "sources").focus();
  for (let index = 0; index < 8; index++) {
    await page.keyboard.press("Tab");
    await expect
      .poll(() =>
        page.evaluate(() => {
          const focused = document.activeElement;
          if (!(focused instanceof HTMLElement)) return false;
          const bounds = focused.getBoundingClientRect();
          const hit = document.elementFromPoint(
            bounds.x + bounds.width / 2,
            bounds.y + bounds.height / 2
          );
          return focused.contains(hit);
        })
      )
      .toBe(true);
  }
});

test("sticky group actions support creation, moving, drag and drop, and removal", async ({
  page,
}) => {
  await start(page);
  await action(page, "research", "add-group");
  await control(page, "textbox", "group-title").fill("New subgroup");
  await control(page.getByRole("dialog"), "button", "save").click();
  const newHeading = page.locator("[data-group-heading]").filter({
    has: page.getByRole("button", { name: "New subgroup", exact: true }),
  });
  await expect(newHeading).toBeVisible();
  await scrollGroup(page, "research", 300);
  await action(page, "methods", "move-shortcut");
  await control(page, "combobox", "shortcut-destination").click();
  await page
    .getByRole("option")
    .and(page.locator('[data-value="work"]'))
    .click();
  await control(page, "button", "confirm-move").click();
  await expect(
    node(page, "work").locator('[data-shortcut-group="methods"]')
  ).toHaveCount(1);
  await scrollGroup(page, "research", 100);
  const source = control(page, "list", "sidebar-shortcuts-list").getByRole(
    "button",
    { name: "Collection 00", exact: true }
  );
  await source.scrollIntoViewIfNeeded();
  await source.dragTo(groupButton(page, "research"));
  await expect(
    source.locator("xpath=ancestor::*[@data-shortcut-group][1]")
  ).toHaveAttribute("data-shortcut-group", "research");
  await expect(
    node(page, "sources").getByRole("button", {
      name: "Collection 00",
      exact: true,
    })
  ).toHaveCount(0);
  await heading(page, "work").scrollIntoViewIfNeeded();
  await action(page, "work", "remove-node");
  await expect(node(page, "work")).toHaveCount(0);
  await expect(node(page, "methods")).toHaveCount(1);
});

test("read-only shortcut groups retain expansion and sticky context", async ({
  page,
}) => {
  await page.goto("/?scenario=read-only&fixture=shortcut-groups");
  await control(
    page.getByRole("dialog"),
    "button",
    "continue-read-only"
  ).click();
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await scrollGroup(page, "research", 300);
  await stacked(page, ["research", "background", "sources"]);
  await expect(
    control(heading(page, "research"), "button", "shortcut-actions")
  ).toBeDisabled();
  await groupButton(page, "research").click();
  await expect(groupButton(page, "research")).toHaveAttribute(
    "aria-expanded",
    "false"
  );
  await groupButton(page, "research").click();
  await expect(groupButton(page, "research")).toHaveAttribute(
    "aria-expanded",
    "true"
  );
});
