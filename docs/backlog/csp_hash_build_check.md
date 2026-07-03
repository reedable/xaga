# Fail the build on CSP hash drift (pending)

The reader iframe's inline script is CSP-hash-allow-listed; the hash is currently regenerated
by hand (`node scripts/csp-hash.mjs`) and pasted into `script-src` in
`src-tauri/tauri.conf.json`. Rot is silent: a drifted hash produces a blank reader in the
bundled app.

Recorded fix direction: an npm script (run in `beforeBuildCommand`) that recomputes the reader
script's sha256 and errors when it doesn't match the CSP.
