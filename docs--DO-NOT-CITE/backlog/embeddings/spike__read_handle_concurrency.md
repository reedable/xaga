# Shared read handle under concurrent load (pending)

`AppState` holds a single shared LanceDB read connection alongside the service's writer (`src-tauri/src/lib.rs`) — one handle, no pooling, no per-request connection. Behavior under concurrent read/write load at the design target (`docs/architecture/design_scale.md`) is unexamined. Related: `db_reload_on_every_read.md` (the reload strategy on this same handle).
