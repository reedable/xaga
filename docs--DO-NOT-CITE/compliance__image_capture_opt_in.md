# Compliance: opt-in article image capture

Implement image capture as opt-in and off by default. Describe the setting as
saving article images for offline viewing. Explain that capture can issue
additional network requests; do not imply that the setting controls the original
page's normal image loading in the article loader.

- Disable Xaga's additional image-capture fetches when the setting is off,
  including in visible loaders.
- Keep already-persisted images usable when capture is disabled.
- Preserve the ability to view the original page and its images through “open
  in loader window” without opting into capture.
- Define how the preference applies to existing installations and verify its
  persistence and behavior on subsequent ingestion and reload.
- Test both preference values, including failed capture and existing local
  images. Do not present enabling main/settings network access as the remedy.

Relevant code: `src-tauri/src/article_loader.js` and the settings pipeline.

Update durable documentation with the capture preference and its effect on
article ingestion and visible loaders.
