# Apply the release DMG background on CI

## Summary

The release DMG built on CI does not display the configured background in
Finder, while a locally built DMG does. Diagnose and fix the shared packaging
job used by Release Candidate and Release.

## Requirements

- A DMG produced by the hosted macOS release job displays the configured
  background and installer icon layout when opened in Finder.
- DMG packaging completes without interactive approval. Bound any Finder
  automation so it fails visibly if the runner cannot complete it.
- Preserve the existing release artifact, signature verification, and
  publication behavior.

## Remaining work

- Inspect an affected build log to determine whether Tauri skipped the Finder
  layout step or attempted it and failed.
- If CI mode skipped the step, test `TAURI_BUNDLER_DMG_IGNORE_CI=true` on the
  shared packaging step. If Finder automation cannot run reliably on the
  hosted runner, evaluate applying the layout metadata without Finder.
- Verify the fix in a hosted macOS run. Download the retained DMG, open it in
  Finder, and capture the background and icon layout. Record the run link and
  confirm that the existing artifact and signature checks pass.

## References

- [Shared release pipeline](../../.github/workflows/release-pipeline.yml)
- [Tauri DMG configuration](../../src-tauri/tauri.conf.json)
- [Tauri's CI override for DMG bundling](https://github.com/tauri-apps/tauri/pull/11799)
- [Hosted runner Finder automation timeout report](https://github.com/tauri-apps/tauri-action/issues/1091)
