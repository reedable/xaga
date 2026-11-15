import {
  control,
  expect,
  startCapture,
  test,
  setSidebarVisible,
  openArticle,
} from "./fixtures";

for (const width of [960, 720]) {
  test(`responsive refresh stays clear of native traffic-light space at width ${width}`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 720 });
    await startCapture(page);
    await setSidebarVisible(page, false);
    await expect(
      control(page, "button", "sidebar-sort-trigger")
    ).not.toBeVisible();
    const refresh = control(page, "button", "refresh-collection");
    const collection = control(page, "combobox", "collection-selector");
    await expect(refresh).toBeVisible();
    await expect(collection).toBeVisible();
    const reserved = (await page
      .getByTestId("traffic-light-spacer")
      .boundingBox())!;
    const button = (await refresh.boundingBox())!;
    const selector = (await collection.boundingBox())!;
    expect(button.x).toBeGreaterThanOrEqual(reserved.x + reserved.width);
    expect(button.x + button.width).toBeLessThanOrEqual(
      selector.x + selector.width
    );
    await refresh.focus();
    await expect(refresh).toBeFocused();
    expect(
      await refresh.evaluate((element) => {
        const box = element.getBoundingClientRect();
        return element.contains(
          document.elementFromPoint(
            box.x + box.width / 2,
            box.y + box.height / 2
          )
        );
      })
    ).toBe(true);
    await refresh.click();
    await expect(collection).toBeVisible();
    const image = testInfo.outputPath(`titlebar-controls-${width}.png`);
    await page.screenshot({ path: image });
    await testInfo.attach("responsive-titlebar", {
      path: image,
      contentType: "image/png",
    });
  });
}

test("disabled responsive refresh keeps its tooltip and traffic-light clearance", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 720 });
  await page.goto("/?scenario=read-only");
  await control(
    page.getByRole("dialog"),
    "button",
    "continue-read-only"
  ).click();
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await setSidebarVisible(page, false);
  await page.setViewportSize({ width: 720, height: 720 });
  const refresh = control(page, "button", "refresh-collection");
  await expect(refresh).toBeDisabled();
  const reserved = (await page
    .getByTestId("traffic-light-spacer")
    .boundingBox())!;
  const button = (await refresh.boundingBox())!;
  expect(button.x).toBeGreaterThanOrEqual(reserved.x + reserved.width);
  await refresh.locator("..").hover();
  await expect(page.getByRole("tooltip")).toBeVisible();
});

for (const width of [1440, 720]) {
  test(`titlebar receives blank-area clicks over every column at width ${width}`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    await startCapture(page);
    if (width === 720) await setSidebarVisible(page, false);
    await openArticle(page, "A garden for every season");
    const bar = page.getByTestId("traffic-light-spacer").locator("..");
    expect(
      await bar.evaluate((element) => {
        const barBox = element.getBoundingClientRect();
        const columns = [
          document.getElementById("sidebar-column"),
          document.getElementById("entity-list-column"),
          document.querySelector('[data-testid="reading-space"]'),
        ].filter(
          (column): column is HTMLElement => column instanceof HTMLElement
        );
        return columns
          .filter((column) => column.getBoundingClientRect().width > 0)
          .map((column) => {
            const box = column.getBoundingClientRect();
            return element.contains(
              document.elementFromPoint(box.x + box.width / 2, barBox.y + 8)
            );
          });
      })
    ).toEqual(width === 1440 ? [true, true, true] : [true, true]);
    // Positioned resize handles must stay above the drag surface.
    for (const id of width === 1440
      ? ["sidebar-column-resize", "entity-list-column-resize"]
      : ["entity-list-column-resize"]) {
      const handle = page.getByTestId(id);
      expect(
        await handle.evaluate((element) => {
          const box = element.getBoundingClientRect();
          return element.contains(
            document.elementFromPoint(box.x + box.width / 2, box.y + 8)
          );
        })
      ).toBe(true);
    }
  });
}
