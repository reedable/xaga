# Call-path review: UI → Tauri → core, CLI → core

Review the two call paths end to end and identify unnecessary code. Overlaps `dead_code_sweep.md` (symbol-level) but is route-level: whole paths that exist without a consumer or duplicate another route. Related standing rule: `docs/architecture/layer_ownership.md`.
