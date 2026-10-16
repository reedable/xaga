# Improvement: reduce loader fingerprinting footprint

## Summary

Reduce unnecessary identifying signals exposed to publisher scripts in the
article loader and shared resource downloader. Prioritize obvious Xaga-specific
signals and avoidable disclosures. The goal is a smaller fingerprinting
footprint, not anonymity or indistinguishability from Safari.

## Requirements

- Do not falsify the user agent to impersonate Safari or another browser.
- Preserve article rendering, capture, image/PDF acquisition, and native request
  attribution. Keep remote execution in the isolated loader.
- Preserve WebRTC and speculative-DNS restrictions, proxy enforcement, and
  session isolation even when those controls make the loader distinguishable.
- Prefer removing unnecessary page-visible implementation details over adding
  spoofed values. Renaming globals alone is not a reliable privacy boundary.
- Assess compatibility and privacy benefit before changing exposed browser or
  device characteristics. Do not assume randomization reduces fingerprintability.
- Keep ephemeral storage as the default and preserve explicit persistent-session
  exceptions. Do not treat discarded storage as protection against fingerprinting.
- Measure improvements with controlled fixtures; do not send device fingerprints
  to public testing services. Distinguish observed signals from claims about
  population uniqueness or reliable user identification.

## Remaining work

- Audit page-visible loader globals, helper functions, and native bridge exposure.
  Determine which can be removed from the publisher's JavaScript world or kept
  in an isolated execution context without breaking capture or IPC attribution.
- Assess hidden-window and viewport disclosures. Identify unnecessary variation
  or implementation signatures and evaluate reductions against responsive layout
  and capture fidelity. Include visible loader behavior before changing sizing.
- Measure the baseline on controlled remote HTTP/HTTPS origins and across fresh
  sessions. Compare ordinary Safari under recorded settings; distinguish built-in
  WebKit behavior from Xaga-specific behavior and the localhost fixture's effects.
- Evaluate remaining exposed characteristics for practical, compatible reductions:
  screen/viewport, language/timezone, hardware information, canvas, audio, font
  measurements, and WebGL. Prioritize demonstrated benefit rather than attempting
  to eliminate every fingerprinting surface.
- Implement selected reductions and verify capture/download behavior, security
  controls, and before/after measurements. Keep raw machine-specific results out
  of committed fixtures and documentation.
- Update durable security documentation with the protections and remaining limits.

## Reference

### Measured baseline

The existing comparison used one machine, ordinary Safari with its existing
preferences, and the same loopback HTTP page in two fresh loader data stores.

- JavaScript and request-header user agents differ: the loader lacks Safari's
  `Version/... Safari/...` suffix. This is an observed distinction, not a request
  to add that suffix.
- `RTCPeerConnection` is unavailable in the loader and constructible in Safari.
  Retain this security restriction.
- The hidden loader reported a 400 × 268 viewport and zero outer dimensions;
  Safari reported its normal window dimensions. These are fixture observations,
  not guarantees across configurations.
- Publisher scripts could see `__LOADER_ID__`, `__LOADER_LABEL__`,
  `__LOADER_URL__`, `__xagaCaptureImage`, and `__TAURI_INTERNALS__`. Xaga-named
  helpers provide a direct implementation signature.
- Canvas and offline-audio hashes, sampled font widths, WebGL details, screen
  information, language, timezone, and hardware concurrency matched Safari.
- All recorded fields matched across both fresh loader stores. This establishes
  stability in that comparison, not uniqueness across users or devices.

### Reproduction and limits

Run from the repository root in a macOS GUI session:

```sh
cargo test -p xaga --test fingerprint_webview --offline
```

The probe opens a Safari tab and saves local measurements to
`/tmp/xaga-fingerprint-comparison.json`. It uses the production loader native
configuration and initialization script, acknowledges capture IPC to retain the
windows, and does not configure the network proxy. The localhost origin is a
secure context. This does not measure normal capture lifetime, remote-origin
privacy behavior, TLS fingerprints, Safari private browsing, or population entropy.

### Code and durable documentation

- `src-tauri/tests/fingerprint_webview.rs`
- `src-tauri/tests/fixtures/fingerprint_probe.js`
- `src-tauri/src/article_loader.rs`, `src-tauri/src/article_loader.js`
- `src-tauri/src/webview_policy.rs`, `src-tauri/src/loader_identity.rs`
- `docs/architecture/security/fetching_remote_content.md`
