# Diagnose blank UI after library-switch relaunch

## Summary

Switching libraries can relaunch into a blank window. The observed case used `tauri dev`; `restart_process` always spawns the current executable even though its comment says development mode should exit for the Tauri watcher. Release behavior has not been established.

## Requirements

- Support a defined library-switch flow in development and bundled builds.
- Release the old library writer and embedding service before a successor attempts the new library.
- Do not spawn a development successor that cannot reach its frontend server.
- Report spawn or startup failure instead of leaving an unexplained blank window.
- Keep code comments aligned with implemented behavior rather than recording intended history.

## Remaining work

- Reproduce library switching separately under `tauri dev` and a bundled release.
- Capture successor startup, frontend-load, and IPC logs to locate the failure boundary.
- If limited to development, choose watcher-driven exit or another explicit development workflow.
- If bundled builds fail, diagnose executable spawning and startup against the updated settings.
- Cover writer, read-only, and shutdown-requested relaunch paths.

## References

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/library.rs`
- `docs/architecture/single_writer_library_lock.md`
