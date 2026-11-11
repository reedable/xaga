# Library upgrade verification

Automated checks cover interrupted file/store/cache checkpoints, mixed-file
failures, retries using a newer target, reader restoration, settings/spawn
failures, and a subprocess reopening the saved library under normal lock checks.
Browser tests cover the startup choice, operation states, availability feedback,
keyboard access, narrow layouts, and both themes without prescribing UI wording.

The desktop smoke test supplements those checks. Use disposable copies of
libraries; retain their complete backups and note the app version and OS.

1. Open an old-layout library. Confirm the explanation appears before model
   startup and offers Upgrade, Choose another Library, and Quit. Escape and
   backdrop clicks must not open browsing. Cancel library selection and confirm
   the gate remains. Separately open a completed new-layout library under another
   writer's lock; Continue read-only must still permit browsing.
2. Open a fresh copy and choose **Upgrade**. Confirm progress remains visible,
   ordinary writes stay disabled, and completion replaces the backup instructions.
   After restart, verify the same library folder opens and counts, reading state,
   colors, annotations, pins, tags and Topic memberships match the source copy.
3. If a machine without an installed model is available, repeat with networking
   disabled: layout upgrade must remain reachable without a download. Separately
   use a completed new-layout library under lock contention to verify **Continue
   read-only** attempts normal startup and reports unavailable features if model
   initialization fails. Do not remove an installed model for this check.
4. Open a disposable new-layout library with a newer completed version or future pending
   target. Choose **Upgrade Xaga** and verify the default browser opens
   `https://github.com/reedable/xaga/releases`. No conversion should start.
5. Repeat the dialog and badge checks in light and dark themes and a narrow window.

Record pass/fail for each step. For a failure, include the action, visible result,
app version, OS, and whether reopening recovers. Native failure injection remains
an automated responsibility; the smoke test does not replace it.
