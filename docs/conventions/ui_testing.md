# UI and UX testing

For every change that affects UI or UX, review the affected user workflows and
browser integration coverage in the same change.

- Update existing browser tests when intended behavior changes. Add meaningful
  coverage for new workflows or gaps exposed by the change, using visible
  controls and user-facing assertions.
- Reuse the capture platform's fixtures, reset, and readiness mechanism. Extend
  those fixtures when needed rather than creating a separate mock backend.
- Run `npm run test:browser` before completing the change. Preserve checks for
  unaffected behavior; do not weaken assertions merely to make the suite pass.
- Identify behavior the browser mock cannot verify, including native IPC,
  persistence, native windows, menus, and external applications. Run the relevant
  component, backend, or native checks and report any remaining verification gap.
- When a UI or UX change needs no browser test changes, explain why the existing
  coverage remains sufficient or which other checks verify the affected behavior.

See [Browser integration tests](../development/browser_tests.md) for setup,
commands, coverage, and failure diagnostics. Browser tests supplement the
existing component and native/backend checks.
