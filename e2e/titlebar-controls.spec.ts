import { control, expect, startCapture, test } from "./fixtures";

for (const width of [960, 720]) {
  test(`responsive refresh stays clear of native traffic-light space at width ${width}`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 720 });
    await startCapture(page);
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
