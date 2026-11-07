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
- Follow the [writing style guide](../../docs/conventions/writing_style.md),
  using the reader-facing language of product training and exact UI labels.
- Describe verified current UI behavior using its actual labels. Distinguish
  implemented behavior from target architecture; omit capabilities that are not
  available. Keep this a simple usage guide rather than a complete feature reference.
- Use concise steps and selected screenshots for workflows that benefit from
  illustration. Give images meaningful alternative text. Any screen recordings
  must have equivalent written steps.
- Use the deterministic dataset and browser capture mode supplied by the
  [MockPlatform story](../active/enhancement__browser-capture-platform.md). Captures must
  not depend on personal library contents or live network responses.
- Verify native-only workflows against the real app and capture native UI there;
  the browser capture mode does not verify native behavior.

## Remaining work

- Walk through the current app and draft the basic workflow instructions.
- Use the browser capture platform once available; drafting can proceed before
  that dependency is complete.
- Capture the selected examples, store guide assets under `docs/images/`, and
  replace the placeholder with the illustrated guide.
- Verify each documented workflow against the app, check reference and asset
  links, and confirm repeated fixture resets reproduce the same example states.

## References

- [User Guide placeholder](../../docs/user_guide.md)
- [Installation](../../docs/installation.md)
- [Library upgrade](../../docs/library_upgrade.md)
- [Organization model](../../docs/architecture/organization_model.md)
- [Interactive training preview](../../docs/training/index.html)
- [Writing style](../../docs/conventions/writing_style.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
- [Browser capture platform](../active/enhancement__browser-capture-platform.md)
