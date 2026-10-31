# Compliance: persisted resource lifecycle

## Summary

Verify private download staging and the persisted-resource paths used by article
favicons and summary/body images. Add missing integration coverage and repair
violations of the existing persistence and local-only presentation contracts.
Verify existing URL-based recovery and explicit article refresh. Introducing
automatic repair of hash-only references or source-URL retention is outside scope.

## Requirements

- Native downloads must use app-generated private temporary paths when macOS
  Downloads access is undecided or denied. Server filenames and completion
  paths must not select the file consumed or the library destination.
- Verify these cases through the existing persistence and presentation contracts:

  | Case | Acceptance condition |
  | --- | --- |
  | Cached reopen, including after restart | Local bytes remain readable; article opening makes no favicon request. |
  | Same favicon URL shared by two articles | Both resolve one resource keyed by normalized URL; successful refresh atomically updates its bytes; failed acquisition or persistence retains existing bytes and the saved reference. |
  | New article without acquired favicon bytes | No broken local reference is introduced. |
  | Legacy entry-local favicon | Cached display works; successful remote refresh stores the shared reference. |
  | Edited remote, missing, foreign-entry, or traversal favicon reference | No direct UI fetch or unauthorized file read; article content remains available. |
  | Known remote summary/body image URL, cached or absent | Reuse local bytes without acquisition; acquire absent bytes only with `load_remote_image` enabled and through the isolated loader. |
  | Missing hash-only local file | Safe unavailable-image or absent-favicon presentation without automatic acquisition; an explicit article refresh can reacquire resources supplied by the refreshed page. Never infer a source URL from a hash. |
  | Failed acquisition or asset write; stale recovery snapshot | Persist bytes before rewriting references; failed references, unrelated fields, existing assets, and concurrent edits survive. |
  | Read-only library, recovery setting on and off | Cached display works; implicit recovery makes no requests; persistence commands enforce write rights; no library files or assets change. |

- Check private staging cleanup on the persistence paths exercised here.

## Remaining work

- Inventory existing persistence, recovery, frontend, and native assertions;
  extend coverage only where the acceptance matrix lacks evidence.
- Use temporary libraries and controlled local endpoints. Assert resource bytes,
  saved references, request counts, and library changes rather than relying on
  screenshots or mocked successful saves. Include production writer/command
  boundaries where unit tests alone cannot establish the requirement.
- Verify Downloads access states in the bundled macOS app, recording the app
  identity, permission state, actual staging destination, outcome, and cleanup.
  A temporary-directory unit test does not establish macOS permission behavior;
  report any unavailable permission-state setup as an explicit verification gap.
- Verify local-only presentation under the bundled app CSP or an equivalent
  native harness using that exact CSP; `tauri dev` alone is insufficient.
- Integrate automated cases into `test:headless` or `test:gui` as appropriate,
  fix demonstrated contract violations, and document verified guarantees and
  limits. Keep any required manual permission-state check explicitly documented.

## References

- [Article persistence](../../crates/xaga-core/src/persistence/articles.rs)
- [Resource storage and reads](../../crates/xaga-core/src/persistence/images.rs)
- [Recovery writes](../../crates/xaga-core/src/persistence/image_recovery.rs)
- [Article and recovery commands](../../src-tauri/src/commands/articles.rs)
- [Validators and staging](../../src-tauri/src/resource_download.rs)
- [Native download tests](../../src-tauri/tests/resource_download_webview.rs)
- [Native lifecycle tests](../../src-tauri/tests/loader_lifecycle_webview.rs)
- [Reader recovery](../../src/components/content/ArticleReader.tsx)
- [Local-only image preparation](../../src/text/prepareEntryImages.ts)
- [Toolbar favicon presentation](../../src/components/content/Toolbar.tsx)
- [Remote-content policy](../../docs/architecture/security/fetching_remote_content.md)
- [Persisted-content policy](../../docs/architecture/security/handling_persisted_content.md)
- [Favicon discovery and cleanup](../backlog/enhancement__favicon--handling.md)
- [Reference-style image recovery](../backlog/enhancement__reference-style-image-recovery.md)
