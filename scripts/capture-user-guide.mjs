// Recreate the User Guide screenshots from the shared capture dataset.
// Start npm run ui:capture first; XAGA_CAPTURE_URL can select another local port.
import { chromium, expect } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const baseURL = process.env.XAGA_CAPTURE_URL ?? "http://127.0.0.1:1420/";
const output = process.argv[2]
  ? resolve(process.argv[2])
  : fileURLToPath(new URL("../docs/images/user-guide/", import.meta.url));
const browser = await chromium.launch();
try {
  for (const theme of ["light", "dark"]) {
    const context = await browser.newContext({
      viewport: { width: 1440, height: 900 },
      deviceScaleFactor: 1,
      colorScheme: theme,
      reducedMotion: "reduce",
      locale: "en-US",
      timezoneId: "UTC",
      serviceWorkers: "block",
    });
    const remote = [];
    await context.route("**/*", async (route) => {
      if (new URL(route.request().url()).origin === new URL(baseURL).origin)
        await route.continue();
      else {
        remote.push(route.request().url());
        await route.abort();
      }
    });
    const page = await context.newPage();
    const reader = (page) =>
      page.getByTitle("Entry reader", { exact: true }).contentFrame();
    await mkdir(output, { recursive: true });
    async function capture(name) {
      await page.mouse.move(0, 0);
      await page.evaluate(() => document.fonts.ready);
      for (const image of await reader(page).locator("img").all())
        await expect(image).toHaveJSProperty("complete", true);
      const screenshot = await page.screenshot({ animations: "disabled" });
      // Frame the captured pixels separately so the application layout and
      // reader interactions stay identical to the unframed capture.
      const frame = await browser.newPage({
        viewport: { width: 1520, height: 980 },
        deviceScaleFactor: 1,
      });
      try {
        await frame.setContent(`
          <style>
            html, body { margin: 0; background: transparent; }
            body { padding: 40px; }
            img {
              display: block; width: 1440px; height: 900px;
              outline: 1px solid ${theme === "dark" ? "rgba(255,255,255,0.28)" : "rgba(0,0,0,0.20)"};
              box-shadow: 0 8px 24px ${theme === "dark" ? "rgba(0,0,0,0.55)" : "rgba(0,0,0,0.20)"};
            }
          </style>
          <img alt="" src="data:image/png;base64,${screenshot.toString("base64")}">
        `);
        await frame.locator("img").evaluate((image) => image.decode());
        await frame.screenshot({
          path: `${output}/${name}${theme === "dark" ? "-dark" : ""}.png`,
          omitBackground: true,
        });
      } finally {
        await frame.close();
      }
      console.log(`Captured ${name} (${theme})`);
    }
    const captureURL = new URL(baseURL);
    captureURL.searchParams.set("theme", theme);
    await page.goto(captureURL.href);
    await expect(page.locator("html")).toHaveAttribute(
      "data-capture-ready",
      "true"
    );
    await page.getByRole("button", { name: "All", exact: true }).click();
    await page
      .getByRole("button", { name: /^A garden for every season / })
      .click();
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      "A garden for every season"
    );
    await expect(
      page.getByRole("listitem").filter({
        has: page.getByRole("button", { name: "All items", exact: true }),
      })
    ).toHaveText("All items1");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await capture("reading");

    await page
      .getByRole("button", { name: "Add bookmark", exact: true })
      .click();
    await page
      .getByRole("dialog", { name: "Bookmark color" })
      .getByRole("button", { name: "blue", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Change bookmark color", exact: true })
      .click();
    await expect(
      page
        .getByRole("dialog", { name: "Bookmark color" })
        .getByRole("button", { name: "blue", exact: true })
    ).toHaveAttribute("aria-pressed", "true");
    await capture("bookmark");
    await page.keyboard.press("Escape");

    await reader(page).locator("article p").first().click({ clickCount: 3 });
    await page.getByRole("button", { name: "violet", exact: true }).click();
    await page
      .getByPlaceholder("Add a note…")
      .fill("Try this on my next walk.");
    await expect(
      reader(page).locator("mark").filter({ hasText: "Small changes" })
    ).toBeVisible();
    await capture("annotation");
    await page.getByRole("button", { name: "Saved", exact: true }).click();
    await expect(
      page.getByRole("button", { name: /^Small changes/ })
    ).toBeVisible();
    await page.getByRole("button", { name: "All", exact: true }).click();

    await page.getByRole("button", { name: /^Following the water / }).click();
    await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
      "Following the water"
    );
    await page.getByRole("tab", { name: "Topics", exact: true }).click();
    await page
      .getByRole("checkbox", { name: "Everyday discoveries", exact: true })
      .check();
    await expect(
      page.getByRole("checkbox", { name: "Everyday discoveries", exact: true })
    ).toBeChecked();
    await expect(
      page.getByRole("listitem").filter({
        has: page.getByRole("button", { name: "All items", exact: true }),
      })
    ).toHaveText("All items");
    await capture("topics");

    await page.getByRole("tab", { name: "Tags", exact: true }).click();
    const tags = page.getByRole("group", { name: "Entry tags", exact: true });
    await tags.getByRole("button", { name: "Outdoors", exact: true }).click();
    await tags.getByRole("button", { name: "Projects", exact: true }).click();
    await expect(
      tags.getByRole("button", { name: "Projects", exact: true })
    ).toHaveAttribute("aria-pressed", "true");
    await capture("tags");

    await page.getByRole("button", { name: "Sites (2)", exact: true }).click();
    await page
      .getByRole("button", { name: "fieldnotes.example", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: /^A garden for every season / })
    ).toBeVisible();

    expect(remote, "Guide examples make no remote requests").toEqual([]);
    await context.close();
  }
} finally {
  await browser.close();
}
