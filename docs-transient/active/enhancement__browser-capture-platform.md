# Browser Capture Platform

## Summary

Let AI assistants run the actual Xaga UI in a browser without Rust, using a
JavaScript `MockPlatform` and deterministic data to capture guide and training
examples.

## Requirements

- Implement `Platform` alongside `TauriPlatform`, selected explicitly for
  development and capture. Normal application startup uses `TauriPlatform`.
- Run the existing React UI without native IPC. Adapt native window identification
  and title-bar handling for browser mode rather than duplicating the UI.
- Supply a fixed synthetic dataset covering feeds, articles, annotations, Topics,
  Sites, tags, and pins, with stable IDs, dates, and local image assets. Runtime
  examples require no remote feeds, article requests, or personal library data.
- Support browsing, adding a predefined feed, changing read status and bookmark
  color, adding annotations, managing Topic membership and tags, and pinning or
  unpinning targets. Use minimal in-memory state and the change events expected
  by existing providers; return updated data and consistent counts after edits.
- Reset the dataset and relevant browser view state to a repeatable initial
  state, including navigation, filters, expansion, and appearance. Generate new
  fixture identities and dates deterministically across resets.
- Document unsupported native operations, including real library selection,
  native menus, and external applications. Unsupported actions must not pretend
  to succeed. Do not implement filesystem persistence, embeddings, or a network
  backend for this story.
- Provide launch, reset, and capture instructions with a fixed viewport, theme,
  fonts, and motion preference, plus an observable condition for completed UI
  loading. The setup supports screenshots and optional recordings; producing
  guide or training assets belongs to their consuming stories.

## Remaining work

- Implement the fixture dataset, mock methods/events, browser startup, and reset.
- Exercise the supported workflows in the browser and verify that reset and
  replay reproduce the same visible states without runtime network requests.
- Run affected frontend checks and verify normal startup still selects the real
  platform. Document capture setup and native-only limitations in durable docs.

## References

- [Simple User Guide](../available/enhancement__simple-user-guide.md)
- [Platform interface](../../src/platform/Platform.ts)
- [TauriPlatform](../../src/platform/TauriPlatform.ts)
- [Application entrypoint](../../src/main.tsx)
- [Title bar](../../src/components/titlebar/TitleBar.tsx)
- [Training material](../../docs/training/README.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
