# Diagnose blank UI after library-switch relaunch

## Summary

Switching libraries under `tauri dev` relaunches into a blank main window.
Opening Settings from the macOS menu after that switch also produces a blank
window. The user reports that switching works in 0.15.0. Investigate the shared
frontend-load and development relaunch path without assuming a confirmed cause.

## Requirements

- Support a defined library-switch flow in development and bundled builds.
- Release the old library writer and embedding service before a successor attempts the new library.
- Do not spawn a development successor that cannot reach its frontend server.
- Report spawn or startup failure instead of leaving an unexplained blank window.
- Keep code comments aligned with implemented behavior rather than recording intended history.

## Remaining work

- Reproduce a library switch under `tauri dev`, then open Settings from the macOS
  menu; verify frontend loading in both windows. Compare with the reported working
  0.15.0 flow.
- Capture successor startup, frontend-load, development-server availability, and
  IPC logs to locate the failure boundary. Check `restart_process` executable
  spawning against its stated watcher-driven development behavior.
- If limited to development, choose watcher-driven exit or another explicit development workflow.
- If bundled builds fail, diagnose executable spawning and startup against the updated settings.
- Cover writer, read-only, and shutdown-requested relaunch paths.

## References

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/library.rs`
- `docs/architecture/single_writer_library_lock.md`
