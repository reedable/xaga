# The Tauri command surface: every registered command is webview-callable

Any JavaScript running in a webview can attempt to invoke a registered command with arbitrary arguments. Tauri capabilities and Xaga's native command gate decide which calls may proceed. Which UI elements invoke a command — a menu item, a button, nothing at all — does not restrict who can attempt the call. The caller must be assumed hostile, including for permitted loader callbacks.

This matters because exploits are chains. A renderer compromise on its own may be contained (sandboxed iframe, CSP, no raw-HTML sinks — see [handling_persisted_content.md](handling_persisted_content.md)); a path-accepting command on its own looks benign because "only the menu flow uses it." A permitted command that accepts an unchecked filesystem path can turn script access into an arbitrary file read or overwrite primitive.

## The rule: segregate commands by the authority of their inputs

Tauri 2.12 enforces capability checks for custom commands from remote origins before Xaga's invoke handler runs. `src-tauri/command_registry.json` is the single registration and classification source. `build.rs` generates the handler expression, native command lists, and application permissions in `OUT_DIR`; duplicate command names fail the build. `capabilities/loader.json` grants only loader callbacks to HTTP/HTTPS origins in `loader-*` windows; `capabilities/trusted-ui.json` grants application commands only to local `main` and `settings` windows. Neither grants remote pages access to the first-party command surface. The Rust window-label and library-write gates remain in force after Tauri's check. Unknown commands are denied by the native gate even in trusted windows.

Add or remove a command in the registry, not generated files or a separate handler list. Review its classification and, independently, whether it belongs in the library-write gate. `src-tauri/tests/loader_acl.rs` exercises every registered command and an unknown command across loader, trusted, and unknown windows, with HTTP, HTTPS, app-local, file, and data origins. It tests Tauri's ACL alone and in combination with the production window-label gate. An independent assertion pins the six reviewed loader callbacks, so broadening remote authority requires an explicit test change. An empty loader capability blocks capture and diagnostic IPC, producing a timeout even when navigation finishes successfully.

## Loader callback ownership

Native loader creation registers the original source URL against its window label in `LoaderSessions`; destruction removes it. This session outlives the first capture for visible loaders. `loader_will_unload`, `loader_page_loaded`, and `capture_and_close_loader` require a registered invoking window, read its current URL from the native webview, and refuse non-HTTP(S) locations. They accept no target label or attribution URL from JavaScript. The close callback closes only its invoking window and computes `samePage` from the native original and current URLs. HTML and other captured content remain untrusted and follow the normal sanitization path; native attribution does not establish the truth of page-supplied content.

`receive_html` and `receive_favicon` retain their invoking-window/request-ID ownership checks. Loader diagnostics derive the request ID from the invoking window. Callback regression tests supply forged identity fields and verify native attribution and rejection after session removal; these tests use Tauri's mock runtime and do not replace a native webview smoke test.

- **Derived-path commands are fine on the invoke surface.** Commands whose filesystem access is computed in Rust from app state (`library_dir` + `{hash12}` filenames, settings paths) take no authority from the caller; the webview choosing _which_ feed to read is harmless because it cannot choose _where_ on disk that resolves to.
- **User-supplied-path commands must not exist on the invoke surface.** Anything that takes a path (or any other ambient-authority handle) from its caller hands that authority to the webview. The file dialog and the operation it authorizes belong together on the Rust side (menu handler → Rust-side dialog → direct call); the chosen path must never round-trip through JS.

## PDF lookup, opening, and cancellation

`read_stored_pdf` and `open_stored_pdf` are trusted-UI commands. Both derive a
library PDF path from a normalized HTTP(S) source URL and validate the stored
bytes. The opener accepts no filesystem path from JavaScript. `cancel_article_load`
is also trusted-only: it cancels the diagnostic attempt, removes matching pending
captures, and destroys their loader windows. The six remote loader callbacks do
not gain access to these operations. PDF writes occur inside `load_article` using
the writer captured with the active library; no writer means a read-only error.

## Where native-orchestrated operations live

User-supplied-path operations live in `src-tauri/src/native/` as plain `pub(crate)` functions, not `#[tauri::command]`s. The menu handler opens the file dialog in Rust (`native::*_dialog`) and calls them directly; none are on the invoke surface, so no path is ever taken from the webview. Completion is reported to the frontend by event (`feeds:imported`, `feeds:exported`, `bookmarks:imported`, `bookmarks:exported`) carrying `{ result?, error? }` — the result on success, the error on failure. Feed and bookmark OPML/HTML import/export are the operations that live here.

The `native/` module is the home for this pattern: anything the app shell triggers that carries ambient authority belongs there, never in `commands/`.

## The trap this guards against

The pull to resist is routing every operation through the JS/webview layer _for consistency_. File import/export is where that pull is strongest — the natural instinct is to hand it to JS because every other operation goes through JS — and the uniformity hides that a trust boundary runs through the middle of it. Consistency across operations is not a virtue at a security boundary. Before registering a new command, ask: does any argument carry authority (a path, a URL to be written to disk as-is, a raw identifier the backend will trust)? If yes, the orchestration belongs in Rust (`src-tauri/src/native/`), not in the webview — however inconsistent that looks next to the other commands.
