# Browser capture

Run the existing React UI against a synthetic, in-memory `MockPlatform` without
Rust, native IPC, a personal library, or external network responses.

## Launch and reset

From the Xaga directory, install the frontend dependencies using the repository's
usual setup, then run:

```sh
npm run ui:capture
```

Open `http://127.0.0.1:1420/`. Capture mode defaults to light; append
`?theme=dark` for dark mode. Reloading the plain URL restores light mode. This command selects Vite's `capture` development
mode explicitly. `npm run ui:dev`, native startup, and production builds select
`TauriPlatform`. Production builds exclude the mock implementation and fixtures.

Reload the page to reset. Automation can call `window.xagaCapture.reset()` and
wait for the new document to load. Reset discards dataset edits, generated IDs,
preferences, filters, sidebar expansion, selection, and navigation history. The
shared sidebar-sort preference is an exception: capture mode stores it in its own
local-storage key to exercise account-preference persistence across reloads.
Fresh browser contexts start with unread sorting. Capture
view state uses memory instead of IndexedDB and does not read or modify saved
native view state. Every launch starts at `/` with no selected article.

Wait for `html[data-capture-ready="true"]` before the first interaction or initial
screenshot. This indicates that settings, collection data, counts, and fonts have
loaded and a rendering frame has run. It is an initial-load condition, not an
indicator that every later interaction has completed. After an action, wait for
its visible result; after opening an article, wait for its iframe's article text
and for images to finish decoding. Collection counts can update through the
existing providers' debounced event handlers.

## Reproducible captures

Use the same browser version and operating system for repeated captures. Set:

- Viewport: 1440 × 900 CSS pixels, device scale factor 1.
- Theme: light (the mock's default); browser color scheme: light.
- Motion preference: `reduce`, set before loading the page.
- Locale: `en-US`; timezone: `UTC`.
- Fonts: the bundled application's CSS font stacks with the mock's initial
  `system` reader family, size 18, and line height 1.6. Wait for
  `document.fonts.ready`; use the same installed system fonts across runs.
- Pointer: move to the same unused corner before screenshots to avoid hover
  differences after replay.

For example, with an independently installed Playwright tool:

```js
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  deviceScaleFactor: 1,
  colorScheme: "light",
  reducedMotion: "reduce",
  locale: "en-US",
  timezoneId: "UTC",
});
const page = await context.newPage();
await page.goto("http://127.0.0.1:1420/");
await page.waitForSelector('html[data-capture-ready="true"]');
await page.screenshot({ path: "capture.png" });
```

Recordings can use the same context settings and observable waits. Producing
User Guide or training assets is separate from maintaining this capture mode.
For automated workflows and CI execution, see
[Browser integration tests](browser_tests.md).

## Example data and workflows

Fixtures live in `src/platform/capture/fixtures.ts`. They contain three articles,
two derived Sites, an annotation, two tags, a Topic, and a pinned Topic shortcut.
Article dates are fixed at `2026-01-15T12:00:00.000Z`. New mock records use that
same timestamp and repeatable identities; passage selection IDs also restart on
reload. Each article uses its own local PNG photo (`garden.png`, `water.png`,
or `wood.png`), supplied as image bytes through the existing reader image API.
No article or feed URL is fetched.

Supported workflows include:

- Browse All items, feeds, Sites, Topics, and tags, with Unread, Saved, and All
  filters; open articles in the reading pane.
- Add the predefined feed `https://workshop.example/feed.xml` through **Add feed**.
  This adds **The workshop**, containing **Making room for a workshop**. Other
  feed URLs are rejected with an explanation.
- Change read status through **Share**, choose a bookmark color, and revisit
  bookmarked articles and annotations in **Saved**.
- Select a passage, choose a highlight color, add a comment, revisit the
  annotation, change its color, or remove it.
- Create Topics, change Topic membership, and browse their members.
- Create, assign, rename, merge, and delete tags.
- Pin or unpin articles and collections through their existing controls and
  observe **Shortcuts** while keeping the reading context.
- Change the supported appearance preferences and use the existing keyboard
  navigation. All edits last only until reload.

## Limits

Real library selection, native menus and windows, operating-system title-bar
controls, opening external applications, live feed refresh, arbitrary URL
loading, PDFs, import/export, pin groups, and privacy/download settings are not
implemented by the mock. Native title-bar gestures are inert in capture mode.
Calls that attempt unsupported operations reject with a capture-mode explanation;
native menu synchronization and close subscriptions have no operating-system
effect. The native app is required to verify those workflows.

The fixture dataset supplies no similarity relationships: Related items and
Topic suggestions return empty results. Text search is unsupported. There is no
embedding model, network backend, or filesystem persistence. Browser captures
verify presentation and in-memory interactions; they do not establish native
behavior or backend correctness.

## User Guide images

The [User Guide](../user_guide.md) uses five screenshots of the shared fictional
library, each captured in light and dark mode. To recreate them, start capture mode as described above and run in a
second terminal:

```sh
npx playwright install chromium
npm run guide:capture
```

`scripts/capture-user-guide.mjs` opens a fresh browser context, walks through the
reading, bookmark, annotation, Topic, and tag examples, and writes PNG files to
`docs/images/user-guide/`. Light images retain the plain filenames; dark images
use a `-dark` suffix. The guide uses `<picture>` with a dark color-scheme source
and a light `<img>` fallback for viewers that do not support theme selection.
Each 1440 × 900 capture is framed with a subtle border, a soft shadow, and
40 pixels of transparent padding, producing a 1520 × 980 PNG. The framing is
baked into the image so it also appears in Markdown viewers without custom CSS. It uses the viewport, appearance, locale, timezone,
fonts, and motion preference described above and waits for the visible results.
The script also visits the example Site. It does not open a personal library or
fetch the fictional feed/article URLs.

To preview regenerated assets without replacing the checked-in images, supply an
output directory:

```sh
npm run guide:capture -- /tmp/xaga-guide-preview
```

If capture mode is running on another port, set `XAGA_CAPTURE_URL` to its local
URL when running the command. Review the generated images alongside the guide
before replacing them. Keep the illustrated controls and example names aligned
with the instructions; the same assets can also be used by the training material.
