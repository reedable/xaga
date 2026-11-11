import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
const output = new URL("./", import.meta.url).pathname;
const browser = await chromium.launch();
const results = [];
try {
  const context = await browser.newContext({
    deviceScaleFactor: 1,
    colorScheme: "light",
    reducedMotion: "reduce",
    locale: "en-US",
    timezoneId: "America/New_York",
  });
  await Promise.all(
    [1512, 1352, 1147, 1024, 840].map(async (width) => {
      for (const [sidebar, list] of [
        [240, 320],
        [360, 320],
        [240, 480],
        [360, 480],
      ]) {
        for (const mode of ["normal", "search", "short"]) {
          const height = mode === "short" ? 400 : 800;
          const page = await context.newPage();
          await page.setViewportSize({ width, height });
          await page.goto(`http://127.0.0.1:1421/?scenario=sidebar-height`);
          await page.waitForSelector('html[data-capture-ready="true"]');
          // Temporary rendered UI overrides only; production sources and visibility rules remain unchanged.
          await page.addStyleTag({
            content: `[class*="_Sidebar_"] { --width: ${sidebar / 16}rem !important; } [class*="_App_"] > [class*="_EntityList_"] { --width: ${list / 16}rem !important; }`,
          });
          await page
            .getByRole("button", { name: /^A garden for every season / })
            .click();
          const reader = page.frameLocator('[data-testid="entry-reader"]');
          await expect(reader.getByRole("heading", { level: 1 })).toHaveText(
            "A garden for every season"
          );
          if (mode === "search") {
            await page.getByTestId("search").click();
            await expect(page).toHaveURL(/search=/);
          }
          const feedToggle = page.getByTestId("sidebar-topics-toggle");
          if (await feedToggle.isVisible()) await feedToggle.click();
          else {
            const selector = page
              .getByTestId("sidebar-list-selector")
              .getByRole("combobox");
            if (await selector.isVisible()) {
              await selector.click();
              await page
                .locator('[role="option"][data-value="TOPICS"]')
                .click();
            }
          }
          await page.evaluate(async () => {
            await document.fonts.ready;
            await new Promise((r) =>
              requestAnimationFrame(() => requestAnimationFrame(r))
            );
          });
          await reader.locator("img").evaluateAll(async (images) => {
            await Promise.all(
              images.map((img) => img.decode().catch(() => {}))
            );
          });
          // Wait for the existing Collapse transitions to reach their final layout.
          await page.waitForFunction(() =>
            [...document.querySelectorAll(".MuiCollapse-root")].every(
              (el) =>
                el.classList.contains("MuiCollapse-entered") ||
                el.classList.contains("MuiCollapse-hidden")
            )
          );
          if (mode === "search") {
            await page.waitForFunction(() => {
              const search = document.querySelector(
                '[class*="_SearchResultList_"]'
              );
              return (
                search &&
                Math.abs(search.getBoundingClientRect().width - 288) < 1 &&
                search.parentElement.classList.contains(
                  "MuiCollapse-wrapperInner"
                ) &&
                search
                  .closest(".MuiCollapse-root")
                  ?.classList.contains("MuiCollapse-entered")
              );
            });
          }
          await page.evaluate(async () => {
            let previous = "",
              stable = 0;
            while (stable < 20) {
              await new Promise((resolve) => requestAnimationFrame(resolve));
              const current = JSON.stringify(
                [
                  ...document.querySelectorAll(
                    'body > *, .MuiCollapse-root, [data-testid="entry-reader"]'
                  ),
                ].map((el) => {
                  const r = el.getBoundingClientRect();
                  return [r.x, r.y, r.width, r.height];
                })
              );
              stable = current === previous ? stable + 1 : 0;
              previous = current;
            }
          });
          await page.mouse.move(0, 0);
          const geometry = await page.evaluate(() => {
            const rect = (selector) => {
              const el = document.querySelector(selector);
              if (!el) return null;
              const r = el.getBoundingClientRect();
              return { x: r.x, y: r.y, width: r.width, height: r.height };
            };
            return {
              sidebar: rect('[class*="_Sidebar_"]'),
              list: rect('[class*="_App_"] > [class*="_EntityList_"]'),
              reader: rect('[data-testid="entry-reader"]'),
              documentWidth: document.documentElement.scrollWidth,
              compact: !!document.querySelector(
                "[data-sidebar-compact-header]"
              ),
            };
          });
          const file = `w${width}-s${sidebar}-l${list}-${mode}.png`;
          await page.screenshot({
            path: output + file,
            animations: "disabled",
          });
          results.push({
            file,
            width,
            height,
            requestedSidebar: sidebar,
            requestedList: list,
            mode,
            ...geometry,
          });
          console.log(file, JSON.stringify(geometry));
          await page.close();
        }
      }
    })
  );
  await writeFile(
    output + "measurements.json",
    JSON.stringify(results, null, 2) + "\n"
  );
  const escape = (s) => s.replaceAll("&", "&amp;").replaceAll("<", "&lt;");
  await writeFile(
    output + "index.html",
    `<!doctype html><meta charset="utf-8"><title>Sidebar width spike</title><style>body{font:16px system-ui;margin:2rem;background:#eee}nav{display:flex;gap:1rem;position:sticky;top:0;background:#eee;padding:1rem}section{margin-bottom:3rem}article{margin:2rem 0}img{max-width:100%;height:auto;border:1px solid #aaa}a{color:#1457a5}</style><h1>Sidebar and EntityList width study</h1><p>60 Chromium captures, light theme, synthetic sidebar-height library, article open, Topics expanded where available. Normal/search: height 800; short: height 400. Current 980/1300 width thresholds and measured compact-height rule are unchanged. Search is open with an empty query; the fixture does not support search results. Widths are CSS pixels at a 16px root font size. Browser captures exclude native window chrome.</p><nav>${[1512, 1352, 1147, 1024, 840].map((w) => `<a href="#w${w}">${w}px</a>`).join("")}</nav>${[
      1512, 1352, 1147, 1024, 840,
    ]
      .map(
        (w) =>
          `<section id="w${w}"><h2>Window width ${w}px</h2>${results
            .filter((r) => r.width === w)
            .map(
              (r) =>
                `<article><h3>Sidebar ${r.requestedSidebar} · EntityList ${r.requestedList} · ${r.mode} · ${r.width} × ${r.height}</h3><p>Rendered Sidebar ${r.sidebar ? r.sidebar.width : "hidden"}; file ${escape(r.file)}</p><a href="${r.file}"><img loading="lazy" src="${r.file}" alt="${r.file}"></a></article>`
            )
            .join("")}</section>`
      )
      .join("")}`
  );
} finally {
  await browser.close();
}
