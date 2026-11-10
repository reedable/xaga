# Library upgrade verification

Automated checks cover interrupted file/store/cache checkpoints, mixed-file
failures, retries using a newer target, reader restoration, settings/spawn
failures, and a subprocess reopening the saved library under normal lock checks.
Browser tests cover the startup choice, operation states, availability feedback,
keyboard access, narrow layouts, and both themes without prescribing UI wording.

The desktop smoke test supplements those checks. Use disposable copies of
libraries; retain their complete backups and note the app version and OS.

1. Open an older library. Confirm the explanation appears before any model
   startup. Choose **Continue read-only**, browse an article, and reopen the
   explanation with the sidebar's **Read-only** button. The button has no tooltip.
2. Open a fresh copy and choose **Upgrade**. Confirm progress remains visible,
   ordinary writes stay disabled, and completion replaces the backup instructions.
   After restart, verify the same library folder opens and counts, reading state,
   colors, annotations, pins, tags and Topic memberships match the source copy.
3. If a machine without an installed model is available, repeat with networking
   disabled: upgrade must remain reachable without a download. **Continue
   read-only** should attempt normal startup and report unavailable features if
   model initialization fails. Do not remove an installed model for this check.
4. Open a disposable library with a newer completed version or future pending
   target. Choose **Upgrade Xaga** and verify the default browser opens
   `https://github.com/reedable/xaga/releases`. No conversion should start.
5. Repeat the dialog and badge checks in light and dark themes and a narrow window.

Record pass/fail for each step. For a failure, include the action, visible result,
app version, OS, and whether reopening recovers. Native failure injection remains
an automated responsibility; the smoke test does not replace it.
