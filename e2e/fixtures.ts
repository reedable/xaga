import { writeFile } from "node:fs/promises";
import { test as base, expect, type Page } from "@playwright/test";

export const test = base.extend<{ browserDiagnostics: void }>({
  browserDiagnostics: [
    async ({ page, context, baseURL }, use, testInfo) => {
      const log: { kind: string; message: string }[] = [];
      const errors: string[] = [];
      const remote: string[] = [];
      page.on("pageerror", (error) => {
        errors.push(error.message);
        log.push({ kind: "pageerror", message: error.stack ?? error.message });
      });
      page.on("console", (message) => {
        if (message.type() === "error" || message.type() === "warning")
          log.push({ kind: message.type(), message: message.text() });
      });
      page.on("requestfailed", (request) =>
        log.push({
          kind: "requestfailed",
          message: `${request.url()}: ${request.failure()?.errorText}`,
        })
      );
      await context.route("**/*", async (route) => {
        const url = new URL(route.request().url());
        if (
          url.origin === new URL(baseURL!).origin ||
          url.protocol === "data:" ||
          url.protocol === "blob:"
        ) {
          await route.continue();
        } else {
          remote.push(url.href);
          log.push({ kind: "remote-request", message: url.href });
          await route.abort("blockedbyclient");
        }
      });
      await use();
      try {
        expect(remote, "No external network requests are allowed").toEqual([]);
        expect(errors, "No unhandled browser errors are allowed").toEqual([]);
      } finally {
        if (
          testInfo.status !== testInfo.expectedStatus ||
          remote.length ||
          errors.length
        ) {
          const logPath = testInfo.outputPath("browser-log.json");
          await writeFile(logPath, JSON.stringify(log, null, 2));
          await testInfo.attach("browser-log", {
            path: logPath,
            contentType: "application/json",
          });
          if (!page.isClosed()) {
            const screenshotPath = testInfo.outputPath("browser-failure.png");
            await page.screenshot({ path: screenshotPath });
            await testInfo.attach("browser-failure", {
              path: screenshotPath,
              contentType: "image/png",
            });
          }
        }
      }
    },
    { auto: true },
  ],
});

export { expect };

export async function startCapture(page: Page) {
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute(
    "data-capture-ready",
    "true"
  );
  await expect(
    page.getByRole("button", {
      name: "A garden for every season",
      exact: false,
    })
  ).toBeVisible();
}

export function articleButton(page: Page, title: string) {
  return page.getByRole("button", { name: new RegExp(`^${title} `) });
}

export function reader(page: Page) {
  return page.getByTitle("Entry reader", { exact: true }).contentFrame();
}

export async function openArticle(page: Page, title: string) {
  await articleButton(page, title).click();
  await expect(reader(page).getByRole("heading", { level: 1 })).toHaveText(
    title
  );
  await expect(
    page.getByRole("button", { name: "Share", exact: true })
  ).toBeEnabled();
}

export async function shareAction(page: Page, name: string) {
  await page.getByRole("button", { name: "Share", exact: true }).click();
  await page.getByRole("menuitem", { name, exact: true }).click();
}
