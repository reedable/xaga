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
- Tauri introduced the CI skip after Finder AppleEvents timed out and failed
  DMG builds. Xaga's shared packaging command inherits that default. The
  pinned Tauri bundler passes `--skip-jenkins` when `CI=true`, which skips the
  AppleScript that sets the background and icon positions.
  [Original Tauri issue](https://github.com/tauri-apps/tauri/issues/592) ·
  [Pinned bundler source](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.4/crates/tauri-bundler/src/bundle/macos/dmg/mod.rs#L183-L191)
- Tauri later added `TAURI_BUNDLER_DMG_IGNORE_CI=true` to opt back into Finder
  automation for DMG packaging. A 2025 report documented a hang at the
  AppleScript step on some GitHub-hosted macOS runners. The report does not
  establish how Xaga's current runner behaves.
  [Tauri override](https://github.com/tauri-apps/tauri/pull/11799) ·
  [Hosted runner report](https://github.com/tauri-apps/tauri-action/issues/1091)
