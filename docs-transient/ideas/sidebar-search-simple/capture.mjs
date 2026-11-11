import { chromium, expect } from "@playwright/test";
import { writeFile } from "node:fs/promises";
const output = new URL("./", import.meta.url).pathname;
const cases = new Map();
function add(width, sidebar, list, search, reason) {
  const key = `w${width}-s${sidebar}-l${list}-${search ? "search" : "normal"}`;
  if (cases.has(key)) cases.get(key).reasons.push(reason);
  else cases.set(key, { key, width, sidebar, list, search, reasons: [reason] });
}
for (const [sidebar, list] of [
  [240, 320],
  [360, 320],
  [240, 480],
  [360, 480],
]) {
  for (const delta of [-1, 0, 1]) {
    add(980 + delta, sidebar, list, false, "Existing 980px Sidebar threshold");
    add(1300 + delta, sidebar, list, true, "Existing 1300px Sidebar threshold");
    add(
      list + 609 + delta,
      sidebar,
      list,
      true,
      "608px article-plus-Search boundary with Sidebar governed by existing thresholds"
    );
  }
  const normalBoundary = sidebar + list + 321;
  if (normalBoundary >= 980)
    for (const d of [-1, 0, 1])
      add(
        normalBoundary + d,
        sidebar,
        list,
        false,
        "Reference width from previous Sidebar rule"
      );
  const searchBoundary = sidebar + list + 609;
  if (searchBoundary >= 1300)
    for (const d of [-1, 0, 1])
      add(
        searchBoundary + d,
        sidebar,
        list,
        true,
        "608px article-plus-Search boundary with Sidebar visible"
      );
}
for (const list of [320, 480])
  for (const width of [840, 900, 920])
    add(width, 240, list, true, "Narrow-window Search reference");
for (const width of [1512, 1352, 1147, 1024, 840])
  for (const [sidebar, list] of [
    [240, 320],
    [360, 320],
    [240, 480],
    [360, 480],
  ])
    add(width, sidebar, list, true, "Original matrix Search reference");
const browser = await chromium.launch();
const results = [];
async function settle(page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    let previous = "",
      stable = 0;
    while (stable < 20) {
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const current = JSON.stringify(
        [
          ...document.querySelectorAll(
            '.MuiCollapse-root,.MuiDrawer-paper,[data-testid="entry-reader"]'
          ),
        ].map((el) => {
          const r = el.getBoundingClientRect();
          return [r.x, r.y, r.width, r.height, el.className];
        })
      );
      stable = current === previous ? stable + 1 : 0;
      previous = current;
    }
  });
}
async function geometry(page) {
  return page.evaluate(() => {
    const rect = (selector) => {
      const el = document.querySelector(selector);
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return { x: r.x, width: r.width, height: r.height };
    };
    return {
      sidebar: rect('[class*="_Sidebar_"]'),
      list: rect('[class*="_App_"] > [class*="_EntityList_"]'),
      reader: rect('[data-testid="entry-reader"]'),
      drawer: rect(".MuiDrawer-paper"),
    };
  });
}
try {
  const queue = [...cases.values()];
  await Promise.all(
    Array.from({ length: 4 }, async () => {
      const context = await browser.newContext({
        deviceScaleFactor: 1,
        colorScheme: "light",
        reducedMotion: "reduce",
        locale: "en-US",
        timezoneId: "America/New_York",
      });
      while (queue.length) {
        const c = queue.shift();
        const page = await context.newPage();
        await page.setViewportSize({ width: c.width, height: 800 });
        await page.goto("http://127.0.0.1:1421/?scenario=sidebar-height");
        await page.waitForSelector('html[data-capture-ready="true"]');
        await page.addStyleTag({
          content: `[class*="_Sidebar_"] { --width: ${c.sidebar / 16}rem !important; } [class*="_App_"] > [class*="_EntityList_"] { --width: ${c.list / 16}rem !important; }`,
        });
        await page
          .getByRole("button", { name: /^A garden for every season / })
          .click();
        const reader = page.frameLocator('[data-testid="entry-reader"]');
        await expect(reader.getByRole("heading", { level: 1 })).toHaveText(
          "A garden for every season"
        );
        const topics = page.getByTestId("sidebar-topics-toggle");
        if (await topics.isVisible()) await topics.click();
        if (c.search) {
          await page.getByTestId("search").click();
          await expect(page).toHaveURL(/search=/);
        }
        await reader
          .locator("img")
          .evaluateAll(async (imgs) =>
            Promise.all(imgs.map((img) => img.decode().catch(() => {})))
          );
        await settle(page);
        const before = await geometry(page);
        const afterSidebar = await geometry(page);
        if (c.search && afterSidebar.reader.width + 288 < 608) {
          await page.evaluate(() =>
            window.dispatchEvent(new Event("capture-search-drawer"))
          );
          await page.locator(".MuiDrawer-paper").waitFor();
          await settle(page);
        }
        const final = await geometry(page);
        await page.mouse.move(0, 0);
        await page.screenshot({
          path: output + c.key + ".png",
          animations: "disabled",
        });
        results.push({
          ...c,
          before,
          afterSidebar,
          final,
          file: c.key + ".png",
        });
        console.log(
          c.key,
          "Sidebar:",
          !!final.sidebar,
          "Search:",
          c.search ? (final.drawer ? "drawer" : "inline") : "closed",
          "reader:",
          final.reader.width
        );
        await page.close();
      }
      await context.close();
    })
  );
  results.sort(
    (a, b) =>
      a.list - b.list ||
      a.sidebar - b.sidebar ||
      a.width - b.width ||
      Number(a.search) - Number(b.search)
  );
  await writeFile(
    output + "measurements.json",
    JSON.stringify(results, null, 2) + "\n"
  );
  await writeFile(
    output + "index.html",
    `<!doctype html><meta charset="utf-8"><title>Sidebar and Search boundaries</title><style>body{font:16px system-ui;background:#eee;margin:2rem}img{max-width:100%;border:1px solid #aaa}article{margin:3rem 0}nav{position:sticky;top:0;padding:1rem;background:#eee;display:flex;gap:1rem}</style><h1>Search drawer: simplified 608px rule</h1><p><a href="../sidebar-search-boundaries/index.html">Previous two-stage rule gallery</a></p><p>All captures are 800px high. Sidebar uses only the existing 980px/1300px thresholds. Search uses a right overlay drawer when article-plus-Search space is below 608px (inline reader below 320px). At exactly 608px Search remains inline. The separate horizontal clipping issue is retained, not corrected. Synthetic library, light theme, empty Search. The drawer is 288px wide with the default MUI backdrop. This is a static screenshot prototype, not a live resize implementation.</p><p><a href="../sidebar-widths/index.html">Original set</a> · <a href="../sidebar-widths-reader-minimum/index.html">Sidebar-only rule set</a></p><nav>${[
      [240, 320],
      [360, 320],
      [240, 480],
      [360, 480],
    ]
      .map(([s, l]) => `<a href="#s${s}l${l}">Sidebar ${s} / List ${l}</a>`)
      .join("")}</nav>${[
      [240, 320],
      [360, 320],
      [240, 480],
      [360, 480],
    ]
      .map(
        ([s, l]) =>
          `<section id="s${s}l${l}"><h2>Sidebar ${s} / EntityList ${l}</h2>${results
            .filter((r) => r.sidebar === s && r.list === l)
            .map(
              (r) =>
                `<article><h3>${r.width} × 800 — Search ${r.search ? (r.final.drawer ? "drawer" : "inline") : "closed"}</h3><p>${r.reasons.join("; ")}. Sidebar ${r.final.sidebar ? "visible" : "hidden"}; reader ${r.final.reader.width}px (underlying layout when drawer open).</p><a href="${r.file}"><img loading="lazy" src="${r.file}" alt="${r.key}"></a></article>`
            )
            .join("")}</section>`
      )
      .join("")}`
  );
  console.log("Total captures:", results.length);
} finally {
  await browser.close();
}
