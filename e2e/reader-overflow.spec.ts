import { test, expect, openArticle, reader, control } from "./fixtures";

const garden = "A garden for every season";

for (const [width, height] of [
  [1512, 948],
  [1512, 500],
  [800, 948],
  [800, 500],
]) {
  test(`populated Inspector keeps the reading pane and column headers within the ${width} × ${height} window`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height });
    await page.goto("/?scenario=reader-overflow");
    await expect(page.locator("html")).toHaveAttribute(
      "data-capture-ready",
      "true"
    );
    await openArticle(page, garden);
    const inspector = page.locator("aside");
    const related = inspector
      .getByRole("tabpanel")
      .filter({ has: page.getByRole("list") });
    await expect(related.getByRole("button")).toHaveCount(12);
    await expect(page.getByTestId("entry-reader")).toBeVisible();
    const geometry = await page
      .getByTestId("entry-reader")
      .evaluate((element) => {
        const frame = element.getBoundingClientRect();
        const pane = document
          .querySelector('[data-testid="article-pane"]')!
          .getBoundingClientRect();
        const root = document.scrollingElement!;
        return {
          frameBottom: frame.bottom,
          paneBottom: pane.bottom,
          viewport: innerHeight,
          pageHeight: root.scrollHeight,
          pageTop: root.scrollTop,
        };
      });
    await testInfo.attach("layout-geometry", {
      body: JSON.stringify(geometry),
      contentType: "application/json",
    });
    expect(geometry.frameBottom).toBeLessThanOrEqual(geometry.viewport);
    expect(geometry.paneBottom).toBeLessThanOrEqual(geometry.viewport);
    expect(geometry.pageHeight).toBeLessThanOrEqual(geometry.viewport);
    await related.getByRole("button").last().scrollIntoViewIfNeeded();
    await expect(related.getByRole("button").last()).toBeInViewport();
    await expect(control(page, "button", "back")).toBeInViewport();
    await expect(control(page, "button", "filter-all")).toBeInViewport();
    await expect(page.getByTestId("entity-list-scroller")).toBeInViewport();
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      garden
    );
    expect(
      await page.evaluate(() => document.scrollingElement!.scrollTop)
    ).toBe(0);
  });
}
