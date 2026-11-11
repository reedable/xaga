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

`npm test` includes this suite along with script, UI, Rust, and native GUI tests.
`npm run test:ci` runs script, UI, Rust, and browser tests without the native GUI suite.

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
  The workflow compares the article button’s accessibility tree before assignment,
  after recoloring, and after removal without prescribing announcement wording.
  Component tests check distinct label text for all six colors; browser coverage
  verifies that the output reaches the accessibility tree.
- Selecting a passage with a real browser click, adding a colored annotation and
  comment, and revisiting its highlighted passage through Saved.
- Adding an article to a Topic and browsing that Topic through the collection
  selector when the responsive layout hides the sidebar.
- Pinning and unpinning an article while preserving the reading context, followed
  by arrow-key navigation through the open list.
- Resetting after edits and restoring the initial dataset, navigation, filters,
  and sidebar expansion.

`e2e/settings.spec.ts` opens Settings through the capture platform’s `window=settings`
parameter and existing readiness/reset mechanism. It checks General/Appearance/
Privacy/Security organization, secure defaults, exact large-integer transport,
redirect validation, successful saving across panel navigation, and refusal/retry.
Component tests cover coordinated close, in-flight saves, invalid input, and
failed drafts; backend tests cover persisted changes and live runtime handles.
The browser mock does not verify native window closing, native IPC, disk writes,
or actual network-policy enforcement.

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

Assertions use visible controls and results through roles and stable control
identities. Field names, selected state, counts and saved fixture data establish
behavior; tests do not depend on English UI labels or explanatory copy. The capture API is used only to
exercise its reset operation; tests do not modify mock data directly. The dataset
and its behavior remain owned by the capture platform.

This suite establishes UI integration with the in-memory platform. Native IPC,
filesystem persistence, backend correctness, and native-only workflows require
the existing component, Rust, and GUI checks. Each suite can also be run individually. Screenshot baseline testing and nested shortcut-group organization
are outside this suite's coverage.

## Failures and CI

The **Verify release** job in `.github/workflows/release.yml` runs browser tests
for both RC tags such as `v0.15.0-rc.1` and final release tags. Branch pushes and
pull requests do not trigger the release workflow.

After lint, formatting, frontend type/CSP checks, and Rust workspace checks pass,
the macOS job installs Chromium and runs `npm run test:headless` followed by
`npm run test:browser` in separate steps, each with a 30-minute timeout. Rust test
compilation therefore does not consume the browser suite’s time allowance. These
commands run the same suites as `npm run test:ci`. Browser failures
block the release build. The job uploads
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

## Native feed-refresh notification

On macOS with a desktop session, start Vite in one terminal:

```sh
npm run ui:dev -- --host 127.0.0.1 --port 1422
```

Then run:

```sh
cargo test -p xaga --test feed_refresh_webview --locked
```

This fixture uses the production native refresh command, IPC adapter, feed hook,
and shared notification component together. A temporary library and controlled
proxy supply one successful feed and one HTTP 403 failure. It checks the visible
failure message, cleared loading, retry, and persistence of the successful feed.
It does not read or modify account settings or user libraries. The fixture page
is test infrastructure and is not an application entry point.

`e2e/annotation-tags.spec.ts` covers annotation tag assignment by keyboard,
creation with immediate assignment, recoloring, passage browsing with retained
tag selection, and read-only assignment display and focus return in a narrow
drawer. These fixtures do not verify native IPC, library locks, Markdown writes,
registry transitions, or upgrades; Rust and platform tests cover those boundaries.

`e2e/library-upgrade.spec.ts` covers read-only badge layout in both themes,
automatic startup explanation and reopening, model-independent upgrade,
read-only continuation, progress polling recovery, failure/retry, restart failure,
unknown counts and memberships, unversioned/interrupted targets, metadata errors
and lock-only status.
Screenshots attached to confirmation tests support visual review. The capture
platform owns these scenarios; no personal library is converted. Native browser
opening, process spawning and disk locks require platform and Rust checks.

Feed-refresh failures use a persistent summary with a scrollable details dialog.
The component test advances timers to verify persistence until dismissal; browser
coverage checks fourteen failure rows, scrolling, both themes, dismissal and retry.
The native feed-refresh fixture opens the production details dialog through the
real refresh command and verifies the failing feed separately from saved successes.
