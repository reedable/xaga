# Simple User Guide

## Summary

Replace the User Guide placeholder with a short, practical introduction to using
Xaga, supported by reproducible UI examples that can also serve the interactive
training material.

## Requirements

- Keep the guide in `docs/user_guide.md`, linked from the existing documentation
  entrypoints. Link to installation and library-upgrade instructions rather than
  duplicating them.
- Cover the basic journey: open or create a library, add a feed, find and read an
  article, use reading state and colored bookmarks, annotate, and organize or
  revisit material through Topics, Sites, Tags, and Shortcuts.
- Describe verified current UI behavior using its actual labels. Distinguish
  implemented behavior from target architecture; omit capabilities that are not
  available. Keep this a simple usage guide rather than a complete feature reference.
- Use concise steps and selected screenshots for workflows that benefit from
  illustration. Give images meaningful alternative text. Any screen recordings
  must have equivalent written steps.
- Use a stable, synthetic dataset for captured examples, reusable across the
  User Guide and training material. Captures must not depend on personal library
  contents, changing remote feeds, or live network responses.
- We might need a `MockPlatform`, implementing `Platform` as a counterpart to
  `TauriPlatform`, so AI assistants can run and interact with the actual UI in a
  browser without the Rust backend and create screenshots and screen recordings
  for the training material. Assess this capture path while preparing the guide.
  If needed, provide the smallest working implementation covering the selected
  workflows, with deterministic fixture data and a reset to the initial state.
  Explicitly identify unsupported native operations and verify descriptions of
  native-only behavior against the real app.
- Keep any mock launch mode explicitly selected for development and capture;
  normal application startup must continue to use `TauriPlatform`.

## Remaining work

- Walk through the current app and draft the basic workflow instructions.
- Establish a reproducible capture setup and fixture dataset; implement the
  minimal `MockPlatform` path if required to let AI assistants capture the UI
  without Rust. Document launch and reset instructions for future captures.
- Capture the selected examples, store guide assets under `docs/images/`, and
  replace the placeholder with the illustrated guide.
- Verify each documented workflow against the app, check reference and asset
  links, and confirm repeated fixture resets reproduce the same example states.
  If mock support is added, run the affected frontend checks and verify normal
  Tauri startup still selects the real platform.

## References

- [User Guide placeholder](../../docs/user_guide.md)
- [Installation](../../docs/installation.md)
- [Library upgrade](../../docs/library_upgrade.md)
- [Organization model](../../docs/architecture/organization_model.md)
- [Interactive training preview](../../docs/training/index.html)
- [Documentation conventions](../../docs/conventions/documentation.md)
- [Platform interface](../../src/platform/Platform.ts)
- [TauriPlatform](../../src/platform/TauriPlatform.ts)
- [Application entrypoint](../../src/main.tsx)
