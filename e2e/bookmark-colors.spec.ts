import {
  test,
  expect,
  startCapture,
  articleButton,
  openArticle,
  shareAction,
} from "./fixtures";

const garden = "A garden for every season";
const water = "Following the water";

for (const theme of ["light", "dark"] as const) {
  test(`bookmark borders retain palette and interaction cues in ${theme} theme`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(60_000);
    await page.emulateMedia({ colorScheme: theme });
    await startCapture(page, theme);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    // Exercise rem dimensions at a larger root size as well as the default.
    const setRootFontSize = async (rootSize: string) => {
      await page.locator("html").evaluate((node, size) => {
        node.style.fontSize = size;
      }, rootSize);
    };
    for (const color of [
      "red",
      "orange",
      "yellow",
      "green",
      "blue",
      "violet",
    ]) {
      await openArticle(page, garden);
      await page
        .getByRole("button", { name: /^(Add bookmark|Change bookmark color)$/ })
        .click();
      await page
        .getByRole("dialog", { name: "Bookmark color" })
        .getByRole("button", { name: color, exact: true })
        .click();
      const row = articleButton(page, garden);
      await expect(row).toHaveAccessibleName(
        new RegExp(`Bookmarked, ${color[0].toUpperCase()}${color.slice(1)}`)
      );
      const checkBorder = async () => {
        const border = await row.evaluate((node) => {
          const style = getComputedStyle(node, "::before");
          const rootSize = parseFloat(
            getComputedStyle(document.documentElement).fontSize
          );
          return {
            width: parseFloat(style.borderLeftWidth) / rootSize,
            color: style.borderLeftColor,
            top: parseFloat(style.top) / rootSize,
            bottom: parseFloat(style.bottom) / rootSize,
          };
        });
        expect(border.width).toBe(0.25);
        expect(border.top).toBe(0);
        expect(border.bottom).toBe(0);
        // Resolve the shared palette value through the browser rather than copying its colors.
        const paletteColor = await row.evaluate((node) => {
          const probe = document.createElement("span");
          probe.style.color = "var(--color-icon)";
          node.append(probe);
          const value = getComputedStyle(probe).color;
          probe.remove();
          return value;
        });
        expect(border.color).toBe(paletteColor);
      };
      await setRootFontSize("1.25rem");
      await checkBorder();
      await setRootFontSize("1rem");
      await checkBorder();
      await shareAction(page, "Mark as unread");
      await expect(row.locator('[data-testid="CircleIcon"]')).toBeVisible();
      await row.focus();
      await page.keyboard.press("Tab");
      await page.keyboard.press("Shift+Tab");
      await expect(row).toBeFocused();
      await expect(row).toHaveClass(/Mui-focusVisible/);
      await row.hover();
      await expect(row).toHaveClass(/Mui-selected/);
      await checkBorder();
      await testInfo.attach(`${color}-selected-unread-focused-hovered`, {
        body: await row.screenshot(),
        contentType: "image/png",
      });
      await openArticle(page, water);
      await expect(row).not.toHaveClass(/Mui-selected/);
      await row.hover();
      await checkBorder();
      await testInfo.attach(`${color}-unselected-hovered`, {
        body: await row.screenshot(),
        contentType: "image/png",
      });
    }
  });
}
