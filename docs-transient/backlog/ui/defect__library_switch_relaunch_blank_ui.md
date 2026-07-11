# Library switch: relaunched window does not render (pending)

Observed by the owner 2026-07-11, during Phase B verification of the runtime-modes work:
switching the library triggers the automatic process relaunch, the new window opens, but
the UI never renders.

## Observed

- Environment: the two-instance verification session (an older-build instance holding
  one library's lock; a `tauri dev` instance as the second). The switch was performed on
  the dev instance. Whether the defect reproduces in a release build is NOT yet known.
- The window reopens (so `restart_process` spawned a successor and the successor reached
  window creation), but the page stays blank/unrendered.

## Facts relevant to diagnosis (verified in code, not yet proven to be the cause)

- `restart_process` (`src-tauri/src/lib.rs`) spawns the current executable and exits —
  unconditionally, in every environment.
- Its own NOTE records a known dev-mode limitation: under `tauri dev` the frontend is
  served by the Vite dev server (`http://localhost:1420/`), which is managed by the
  tauri-cli watcher and "dies with the binary." The NOTE claims "callers fall through to
  a clean exit in that environment," but no dev-mode detection exists in the function —
  the spawn happens regardless. A spawned successor in dev boots against a dev URL whose
  server may be gone → a window with nothing to load. This matches the observed symptom
  but is a HYPOTHESIS until reproduced/instrumented.
- The relaunch flow is exercised from three paths: `set_library` with a running service
  (drain → restart), `set_library` on a read-only instance (direct restart — added in
  Phase B), and the window-close restart path (`shutdown_with_overlay`).

## The ask

1. Reproduce and split by environment: dev vs release build. If release relaunch renders
   correctly, this is the recorded dev-mode limitation surfacing through the new
   read-only `set_library` path, and the fix is dev-mode handling (e.g. detect dev and
   exit cleanly for the watcher to relaunch, or document the manual dev workflow).
2. If the release build also fails to render, this is a new defect in the relaunch flow
   itself — diagnose from the successor's log (startup lines present? IPC arriving?).
3. Either way, make `restart_process`'s NOTE truthful: it currently describes a
   fall-through that the code does not implement.
