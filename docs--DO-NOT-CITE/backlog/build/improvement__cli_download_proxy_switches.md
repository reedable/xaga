# CLI download-model: switches for the proxy settings

## Current behavior

- The app's model download dials through the app's proxy, which reads
  `http_allowed` and `connect_nonstandard_ports_allowed` live from
  settings.json per request.
- `xaga-cli embeddings download-model` starts its own proxy instance with both
  values hardcoded `false` (`crates/xaga-cli/src/cmd/embeddings.rs:293-298`);
  settings.json is not consulted. The code comment discloses the rationale:
  the download target is https on 443, so there is nothing to relax.

## Work

Add CLI switches to `download-model` so the user can control both behaviors
per invocation (e.g. `--allow-http`,
`--allow-nonstandard-connect-ports`). The secure defaults stay as implemented:
both `false` when the switches are not passed.

## Questions

- Switch naming/shape (boolean flags vs. values).
- Whether future network-touching CLI commands share the same switches (only
  `download-model` fetches today).
