# The Tauri command surface: every registered command is webview-callable

Any JavaScript running in a webview can attempt to invoke a registered command with arbitrary arguments. Tauri capabilities and Xaga’s native command checks decide which calls may proceed. Which UI elements invoke a command — a menu item, a button, nothing at all — does not restrict who can attempt the call. The caller must be assumed hostile, including for permitted loader callbacks.

A compromised renderer may be restricted by its iframe sandbox, CSP, and safe HTML rendering (see [handling_persisted_content.md](handling_persisted_content.md)). A permitted command accepting an unchecked filesystem path can bypass that restriction by letting the script read or overwrite files. Intended use by a menu does not restrict other callers.

## Keep caller-selected filesystem access out of the invoke API

Tauri 2.12 enforces capability checks for custom commands from remote origins before Xaga's invoke handler runs. `src-tauri/command_registry.json` is the single registration and classification source. `build.rs` generates the handler expression, native command lists, and application permissions in `OUT_DIR`; duplicate command names fail the build. `capabilities/loader.json` grants only loader callbacks to HTTP/HTTPS origins in `loader-*` windows; `capabilities/trusted-ui.json` grants application commands only to local `main` and `settings` windows. Neither grants remote pages access to application commands. Rust still checks the window label and library-write permission after Tauri’s capability check. It rejects unknown commands even in trusted windows.

Add or remove a command in the registry, not generated files or a separate handler list. Review its classification and, independently, whether it belongs in the library-write check. `src-tauri/tests/loader_acl.rs` exercises every registered command and an unknown command across loader, trusted, and unknown windows, with HTTP, HTTPS, app-local, file, and data origins. It tests Tauri's ACL alone and in combination with the production window-label check. A separate assertion lists the seven reviewed loader callbacks; allowing remote callers more commands requires an explicit test change. An empty loader capability blocks capture and diagnostic IPC, producing a timeout even when navigation finishes successfully.

## Loader callback ownership

Native loader creation registers the original source URL against its window label in `LoaderSessions`; destruction removes it. This session outlives the first capture for visible loaders. `loader_will_unload`, `loader_page_loaded`, and `capture_and_close_loader` require a registered invoking window, read its current URL from the native webview, and refuse non-HTTP(S) locations. They accept no target label or attribution URL from JavaScript. The close callback closes only its invoking window and computes `samePage` from the native original and current URLs. HTML and other captured content remain untrusted and follow the normal sanitization path; native attribution does not establish the truth of page-supplied content.

`receive_html`, `receive_favicon`, and `receive_image` retain their invoking-window/request-ID ownership checks. Loader diagnostics derive the request ID from the invoking window. Callback regression tests supply forged identity fields and verify native attribution and rejection after session removal; these tests use Tauri's mock runtime and do not replace a native webview smoke test.

- **Rust-derived paths can be used by invoke commands.** Rust calculates paths from app state, such as `library_dir` and `{hash12}` filenames or the settings path. The webview may select a feed but cannot choose an arbitrary disk path.
- **Do not accept user-selected file paths through invoke.** A caller-selected path or unrestricted resource handle would let JavaScript choose the resource accessed. Keep the native file dialog and its authorized operation together in Rust (menu handler → native dialog → direct call). Do not send the selected path through JavaScript.

## PDF lookup, opening, and cancellation

`read_stored_pdf` and `open_stored_pdf` are trusted-UI commands. Both derive a
library PDF path from a normalized HTTP(S) source URL and validate the stored
bytes. The opener accepts no filesystem path from JavaScript. `cancel_article_load`
is also trusted-only: it cancels the diagnostic attempt, removes matching pending
captures, and destroys their loader windows. The seven remote loader callbacks do
not gain access to these operations. PDF writes occur inside `load_article` using
the writer captured with the active library; no writer means a read-only error.

## Where native file operations live

User-supplied-path operations live in `src-tauri/src/native/` as plain `pub(crate)` functions, not `#[tauri::command]`s. The menu handler opens the file dialog in Rust (`native::*_dialog`) and calls them directly; none are on the invoke surface, so no path is ever taken from the webview. Completion is reported to the frontend by event (`feeds:imported`, `feeds:exported`, `bookmarks:imported`, `bookmarks:exported`) carrying `{ result?, error? }` — the result on success, the error on failure. Feed and bookmark OPML/HTML import/export are the operations that live here.

Use `native/` for app-shell operations that act on a resource selected through native controls, rather than exposing that resource selection through `commands/`.

## Review new commands for caller-controlled access

Do not route import/export through JavaScript merely to match other operations. Before registering a command, check whether an argument lets its caller choose a file or other resource the backend would access without independent validation. Paths, URLs used directly as output paths, and unchecked identifiers can grant that access. Keep operations requiring native resource selection in `src-tauri/src/native/`.

## UI error delivery

`take_ui_errors` is a trusted-UI command. It takes no window label from
JavaScript: the invoking native window determines which notification queue is
drained. Remote loader capabilities do not include it. Live `app:error` events
are likewise targeted to the originating trusted webview window.


`recover_entry_images` is trusted-UI-only. Its image URLs authorize isolated
network acquisition, not caller-selected filesystem writes. Entry IDs and URL
hashes derive persistence paths. The command checks the library writer before
acquisition and skips recovery in read-only mode; the writer operation rechecks
the targeted content snapshot before committing. `receive_image` is a loader
callback only, validates bytes, and never writes to the library itself.
