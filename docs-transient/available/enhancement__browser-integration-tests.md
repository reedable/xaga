# Browser Integration Tests

## Summary

Automate representative UI workflows in a real browser against `MockPlatform`,
without Rust or native IPC. Depends on the
[Browser Capture Platform](../../docs/development/browser_capture.md).

## Requirements

- Add a browser automation runner that launches the actual React UI in explicit
  mock mode. Reuse the capture platform's dataset, reset, and readiness mechanism;
  do not create a separate mock backend or duplicate fixture behavior.
- Isolate each test's mock and browser state. Wait for observable UI conditions
  rather than fixed delays. Tests require no personal library or external network
  responses; localhost requests for the application and fixture assets are allowed.
- Cover these workflows through visible controls and user-facing assertions:
  - Open an article, change its read status, and verify its list state and counts.
  - Bookmark an article with a color, find it in Saved, and remove the bookmark.
  - Add an annotation and revisit its passage through the UI.
  - Add an article to a Topic and verify it appears when browsing that Topic.
  - Pin and unpin a target and verify Shortcuts updates without clearing the
    current reading context.
- Exercise keyboard navigation and navigation with the sidebar hidden in the
  representative workflows. Scope assertions to implemented behavior; nested
  group organization is not a prerequisite for this suite.
- Provide a documented local command and CI execution. Fail the run on failed
  assertions or startup failure, and retain useful failure artifacts such as
  screenshots, browser errors, and an interaction trace or equivalent diagnostics.
- Keep existing component tests and native/backend checks. This suite verifies
  UI integration against the mock; it does not establish native behavior or
  backend correctness. Screenshot baseline testing is outside this story.

## Remaining work

- Configure the runner, mock-mode server lifecycle, isolated reset, and readiness.
- Implement the workflow tests, local command, CI job, and failure artifacts.
- Verify repeated runs start cleanly, reset prevents state leakage, and a
  deliberately failing assertion produces a failed run with diagnostic artifacts.
- Run affected frontend checks and document execution and coverage limits.

## References

- [Browser Capture Platform](../../docs/development/browser_capture.md)
- [Platform interface](../../src/platform/Platform.ts)
- [Existing component test configuration](../../vitest.config.ts)
- [Package scripts](../../package.json)
- [CI workflows](../../.github/workflows/)
- [List behavior](../../docs/architecture/list_view.md)
- [Pin-store contract](../../docs/architecture/pins.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
