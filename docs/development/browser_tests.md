# Browser integration tests

The Playwright suite runs the actual React UI against the
[browser capture platform](browser_capture.md), using its existing fixtures,
reset operation, and readiness marker. It requires no Rust build, personal
library, or external network responses during the tests.

Follow the [UI and UX testing convention](../conventions/ui_testing.md) when
changing user-facing behavior or interaction.

## Run locally

From the Xaga directory:

```sh
npm ci
npx playwright install chromium
npm run test:browser
```

On Linux, install the browser's operating-system dependencies as well:

```sh
npx playwright install --with-deps chromium
```

Dependency and browser installation require network access. Test interactions
allow only the local application origin and embedded resource bytes. Unexpected
external requests are blocked and fail the test; unhandled browser errors also
fail it. Browser console warnings and errors are retained in failure diagnostics.

Playwright starts `npm run ui:capture -- --port 1421`, waits for the server, and
stops it after the run. Port 1421 must be free. The runner refuses to reuse an
existing server so an unrelated app or native development server cannot silently
supply the test UI. Startup failure fails the command.

Each test receives a fresh browser context and page, starts a fresh in-memory
mock dataset, and waits for `html[data-capture-ready="true"]`. Later steps wait
for visible UI results, including article headings, counts, highlights, and
collection membership. Tests do not use fixed delays. Viewport, locale, timezone,
color scheme, and motion preference are fixed in `playwright.config.ts`.

Run one workflow or check repeated isolation with:

```sh
npm run test:browser -- --grep "Topic membership"
npm run test:browser -- --repeat-each=2
```

The suite runs Chromium with two workers and no automatic retries. Update the
installed browser with `npx playwright install chromium` after upgrading the
locked Playwright dependency. `npm run typecheck` includes the browser tests and
runner configuration.

## Coverage

`e2e/workflows.spec.ts` exercises:

- Keeping All items above the collection filter across section changes and
  unmatched filters, with keyboard navigation and selected-state feedback.
- Opening an article, changing read status, and checking Unread membership and
  aggregate/feed counts after the providers update.
- Choosing and changing a bookmark color, verifying accessible labels and article
  text alignment, revisiting the article in Saved, and removing the bookmark.
- Selecting a passage with a real browser click, adding a colored annotation and
  comment, and revisiting its highlighted passage through Saved.
- Adding an article to a Topic and browsing that Topic through the collection
  selector when the responsive layout hides the sidebar.
- Pinning and unpinning an article while preserving the reading context, followed
  by arrow-key navigation through the open list.
- Resetting after edits and restoring the initial dataset, navigation, filters,
  and sidebar expansion.

`e2e/bookmark-colors.spec.ts` checks all six article bookmark borders in light
and dark themes with selection, unread, hover, and keyboard-focus cues. It attaches
row screenshots for visual review. The capture startup helper accepts a theme
through the existing capture URL parameter.

`e2e/annotation-routing.spec.ts` exercises recoloring and deleting selected
annotations in article, All items, and Topic lists, and editing an unselected
annotation while browsing a Feed. It checks selection and retained reading/list
context. Component tests cover selected annotation routes in all four contexts,
history replacement, and late events; Feed lists expose article items in the UI.

`e2e/filter-history.spec.ts` checks Topic/article Back and Forward after filter
changes and read-status updates. It verifies that filters do not add history
entries, that the current filter survives history and collection navigation, and
that a restored article can remain open while excluded from the Unread list.
It also checks history-only scrolling with retained focus, and unchanged list
scroll position after clicking an entity and rendering its article.
Collection browsing is checked against the complete browser history state:
switching collections leaves it unchanged, and Back/Forward restores the
collection captured when each article was opened, even after intervening browsing.

`e2e/sidebar-sort.spec.ts` checks the shared sort control's placement and alignment,
menu placement beside the sidebar without covering its collections, keyboard operation, checked choice, ordering across sections, and saved preference after
reload. Provider tests cover native-event synchronization, startup races, and
failed saves. Projection tests cover nested groups, count updates, and unchanged
stored order. The native `sidebar_sort_menu` test verifies macOS menu construction,
checkmark updates, enabled commands, and Cocoa action dispatch.

Assertions use the visible controls and results. The capture API is used only to
exercise its reset operation; tests do not modify mock data directly. The dataset
and its behavior remain owned by the capture platform.

This suite establishes UI integration with the in-memory platform. Native IPC,
filesystem persistence, backend correctness, and native-only workflows require
the existing component, Rust, and GUI checks. Those commands remain separate and
unchanged. Screenshot baseline testing and nested shortcut-group organization
are outside this suite's coverage.

## Failures and CI

The **Verify release** job in `.github/workflows/release.yml` runs browser tests
for RC tags such as `v0.15.0-rc.1`. Branch pushes and pull requests do not trigger
the release workflow; final release tags skip its browser steps.

After lint, formatting, frontend type/CSP checks, and script/UI tests pass, the
macOS job installs Chromium and runs the same local browser command. Browser
failures block the remaining Rust checks and the release build. The job uploads
`playwright-report/` and `test-results/` as the `browser-integration` artifact,
retained for 14 days, including when browser tests fail. Artifact upload does not
clear the failure.

For a failed browser test, `test-results/` contains screenshots, a trace, browser
console/error/request diagnostics in `browser-log.json`, and Playwright's error
context. The HTML report links these artifacts:

```sh
npx playwright show-report
npx playwright show-trace test-results/<failed-test>/trace.zip
```

Inspect the failing assertion and error context first, then the browser log and
trace for the preceding interactions. Startup failures appear in the command or
CI step output; they may occur before a browser can produce artifacts. Reports
and test results are ignored by Git and replaced on subsequent runs, so copy any
diagnostics you need before rerunning.
