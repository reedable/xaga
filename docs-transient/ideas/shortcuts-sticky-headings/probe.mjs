import { chromium } from "playwright-core";
import { mkdir, writeFile } from "node:fs/promises";
await mkdir("test-results/sticky-spike", { recursive: true });
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1440, height: 900 },
  reducedMotion: "reduce",
});
const results = [];
page.on("pageerror", (e) => results.push({ pageError: e.message }));
await page.goto("http://127.0.0.1:1421/?scenario=sticky-spike");
await page.locator("html[data-capture-ready=true]").waitFor();
if (
  (await page
    .getByTestId("sidebar-shortcuts-toggle")
    .getAttribute("aria-pressed")) !== "true"
)
  await page.getByTestId("sidebar-shortcuts-toggle").click();
await page.locator('[data-target-title="Research"]').waitFor();
const row = (title) =>
  page
    .locator(`[data-target-title="${title}"]`)
    .locator("xpath=ancestor::div[1]");
const frame = () =>
  page.evaluate(
    () =>
      new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))
  );
await page.evaluate(() => {
  const list = document.querySelector('[data-testid="sidebar-shortcuts-list"]');
  let scroll = list.parentElement;
  while (getComputedStyle(scroll).overflowY !== "auto")
    scroll = scroll.parentElement;
  window.spikeScroll = scroll;
  window.spikeInstall = () => {
    const rows = [...list.querySelectorAll('[class*="NodeRow"]')].filter((r) =>
      r.querySelector(":scope > [aria-expanded]")
    );
    for (const r of rows) r.dataset.spikeGroup = "true";
    for (const r of rows) {
      let parent = r.parentElement.parentElement,
        offset = 0,
        depth = 0;
      while (parent && parent !== scroll) {
        if (parent.matches('[class*="TreeNode"]')) {
          const ancestor = parent.querySelector(":scope > [data-spike-group]");
          if (ancestor) {
            offset += ancestor.getBoundingClientRect().height;
            depth++;
          }
        }
        parent = parent.parentElement;
      }
      Object.assign(r.style, {
        position: "sticky",
        top: `${offset / parseFloat(getComputedStyle(document.documentElement).fontSize)}rem`,
        zIndex: String(100 - depth),
        background: "var(--lliw-semanticColor__background-color--primary)",
      });
      r.dataset.spikeOffset = String(offset);
    }
  };
  window.spikeInstall();
});
async function scrollTo(title, delta = 0) {
  await row(title).evaluate((r, delta) => {
    window.spikeScroll.scrollTop +=
      r.parentElement.getBoundingClientRect().top -
      window.spikeScroll.getBoundingClientRect().top +
      delta;
  }, delta);
  await frame();
}
async function snapshot(name, titles) {
  const data = await page.evaluate((titles) => {
    const s = window.spikeScroll.getBoundingClientRect();
    return {
      scrollTop: window.spikeScroll.scrollTop,
      viewport: { top: s.top, height: s.height },
      rows: titles.map((title) => {
        const a = [...document.querySelectorAll("[data-target-title]")].find(
          (a) => a.dataset.targetTitle === title
        );
        const r = a.closest('[class*="NodeRow"]'),
          b = r.getBoundingClientRect();
        return {
          title,
          top: b.top,
          bottom: b.bottom,
          height: b.height,
          offset: r.dataset.spikeOffset,
        };
      }),
    };
  }, titles);
  results.push({ name, ...data });
  console.log(name, JSON.stringify(data));
}
const long = "Background with a long title that wraps over several lines";
await scrollTo("Research", 240);
await snapshot("nested-sticky", ["Research", long, "Methods"]);
await page.screenshot({
  path: "test-results/sticky-spike/nested.png",
  timeout: 10000,
});
await scrollTo("Methods", 200);
await snapshot("sibling-transition", ["Research", long, "Methods"]);
await scrollTo("Work", 200);
await snapshot("root-release", ["Research", "Work"]);
await scrollTo("Research", 0); // sticky released above; use node natural geometry instead
await page.evaluate(() => {
  const a = document.querySelector('[data-target-title="Research"]');
  window.spikeScroll.scrollTop +=
    a.closest('[class*="TreeNode"]').getBoundingClientRect().top -
    window.spikeScroll.getBoundingClientRect().top +
    160;
});
await frame();
// Show why a constant row-height offset cannot accommodate wrapped titles.
results.push({
  name: "wrapped-parent-height",
  height: await row(long).evaluate((r) => r.getBoundingClientRect().height),
});
// Native keyboard focus scrolling may place a shortcut under the sticky rows.
await row("Specimen 15").getByRole("button").first().focus();
await frame();
results.push({
  name: "focus-occlusion",
  ...(await row("Specimen 15").evaluate((r) => {
    const b = r.getBoundingClientRect();
    const hit = document.elementFromPoint(
      b.x + Math.min(50, b.width / 2),
      b.y + b.height / 2
    );
    return { top: b.top, height: b.height, unobscured: r.contains(hit) };
  })),
});
await scrollTo("Research", 240);
await row("Specimen 00").getByRole("button").first().focus();
await frame();
results.push({
  name: "focus-earlier-shortcut",
  ...(await row("Specimen 00").evaluate((r) => {
    const b = r.getBoundingClientRect();
    const hit = document.elementFromPoint(b.x + 50, b.y + b.height / 2);
    return { top: b.top, height: b.height, unobscured: r.contains(hit) };
  })),
});
// Original menu and collapse controls on sticky headings.
await scrollTo("Methods", 100);
await row("Research").hover();
await row("Research").getByTestId("shortcut-actions").click();
results.push({
  name: "menu",
  visible: await page.getByRole("menu").isVisible(),
});
await page.keyboard.press("Escape");
await row("Research").locator(":scope > [aria-expanded]").click();
await page.waitForFunction(
  () =>
    document
      .querySelector('[data-target-title="Research"]')
      .closest('[class*="NodeRow"]')
      .querySelector(":scope > [aria-expanded]")
      .getAttribute("aria-expanded") === "false"
);
await frame();
await snapshot("collapsed", ["Research", "Work"]);
await row("Research").locator(":scope > [aria-expanded]").click();
await page.getByTestId("filter-collections").fill("Specimen 15");
await page.waitForFunction(
  () =>
    ![...document.querySelectorAll("[data-target-title]")].some(
      (x) => x.dataset.targetTitle === "Work"
    )
);
await page.evaluate(() => window.spikeInstall());
await snapshot("filtered", ["Research", long, "Specimen 15"]);
await page.getByTestId("filter-collections").fill("");
await page.locator('[data-target-title="Work"]').waitFor();
await page.evaluate(() => window.spikeInstall());
await page.setViewportSize({ width: 1440, height: 500 });
await page.evaluate(() => {
  window.spikeInstall();
  const a = document.querySelector('[data-target-title="Level 1"]');
  window.spikeScroll.scrollTop +=
    a.closest('[class*="TreeNode"]').getBoundingClientRect().top -
    window.spikeScroll.getBoundingClientRect().top +
    400;
});
await frame();
await snapshot("deep-short-viewport", ["Level 1", "Level 24", "Specimen 60"]);
await page.evaluate(() => {
  const r = document
    .querySelector('[data-target-title="Specimen 60"]')
    .closest('[class*="NodeRow"]');
  window.spikeScroll.scrollTop +=
    r.getBoundingClientRect().top -
    window.spikeScroll.getBoundingClientRect().top;
});
await frame();
await snapshot("deep-at-shortcuts", [
  "Level 1",
  "Level 5",
  "Level 24",
  "Specimen 60",
]);
await page.screenshot({
  path: "test-results/sticky-spike/deep.png",
  timeout: 10000,
});
await page.setViewportSize({ width: 1440, height: 900 });
await scrollTo("Research", 240);
await page.emulateMedia({ reducedMotion: "no-preference" });
await row("Research").locator(":scope > [aria-expanded]").click();
await page.waitForFunction(() => {
  const n = document
    .querySelector('[data-target-title="Research"]')
    .closest('[class*="TreeNode"]');
  return n
    .querySelector(":scope > .MuiCollapse-root")
    ?.classList.contains("MuiCollapse-hidden");
});
await row("Research").locator(":scope > [aria-expanded]").click();
const transition = await page.evaluate(async () => {
  const n = document
    .querySelector('[data-target-title="Research"]')
    .closest('[class*="TreeNode"]');
  const c = n.querySelector(":scope > .MuiCollapse-root");
  const samples = [];
  for (let i = 0; i < 60; i++) {
    await new Promise(requestAnimationFrame);
    const style = getComputedStyle(c);
    samples.push({
      overflow: style.overflow,
      height: style.height,
      entered: c.classList.contains("MuiCollapse-entered"),
    });
    if (c.classList.contains("MuiCollapse-entered")) break;
  }
  return { samples: samples.length, first: samples[0], last: samples.at(-1) };
});
results.push({ name: "animated-expand", ...transition });
await writeFile(
  "test-results/sticky-spike/results.json",
  JSON.stringify(results, null, 2)
);
console.log(JSON.stringify(results, null, 2));
await browser.close();
