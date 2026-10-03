# The Tauri command surface: every registered command is webview-callable

Every `#[tauri::command]` listed in the `invoke_handler` (`src-tauri/src/lib.rs`) is callable from **any** JavaScript running in the webview, at any time, with **any** arguments. Which UI elements invoke a command — a menu item, a button, nothing at all — does not restrict who can call it. The menu is a convenience for the user; the invoke surface is the actual boundary, and the caller on the other side of it must be assumed hostile.

This matters because exploits are chains. A renderer compromise on its own may be contained (sandboxed iframe, CSP, no raw-HTML sinks — see [handling_persisted_content.md](handling_persisted_content.md)); a path-accepting command on its own looks benign because "only the menu flow uses it." Chained, they are a weapon: whoever gets script into the webview gets every registered command as a local API, and a command that accepts a filesystem path becomes an arbitrary file read or overwrite primitive.

## The rule: segregate commands by the authority of their inputs

Tauri 2.12 enforces capability checks for custom commands from remote origins before Xaga's invoke handler runs. `src-tauri/permissions/application.json` separates loader callbacks from first-party application commands. `capabilities/loader.json` grants only those callbacks to HTTP/HTTPS origins in `loader-*` windows; `capabilities/trusted-ui.json` grants application commands only to local `main` and `settings` windows. Neither grants remote pages access to the first-party command surface. The existing Rust window-label and library-write gates remain in force after Tauri's check.

Keep the application permission lists in sync when adding or removing registered commands. `src-tauri/tests/loader_acl.rs` exercises Tauri's actual capability resolution for loader, trusted, and unknown windows. An empty loader capability is insufficient: it blocks capture and diagnostic IPC, producing a capture timeout even when navigation finishes successfully.

- **Derived-path commands are fine on the invoke surface.** Commands whose filesystem access is computed in Rust from app state (`library_dir` + `{hash12}` filenames, settings paths) take no authority from the caller; the webview choosing _which_ feed to read is harmless because it cannot choose _where_ on disk that resolves to.
- **User-supplied-path commands must not exist on the invoke surface.** Anything that takes a path (or any other ambient-authority handle) from its caller hands that authority to the webview. The file dialog and the operation it authorizes belong together on the Rust side (menu handler → Rust-side dialog → direct call); the chosen path must never round-trip through JS.

## Where native-orchestrated operations live

User-supplied-path operations live in `src-tauri/src/native/` as plain `pub(crate)` functions, not `#[tauri::command]`s. The menu handler opens the file dialog in Rust (`native::*_dialog`) and calls them directly; none are on the invoke surface, so no path is ever taken from the webview. Completion is reported to the frontend by event (`feeds:imported`, `feeds:exported`, `bookmarks:imported`, `bookmarks:exported`) carrying `{ result?, error? }` — the result on success, the error on failure. Feed and bookmark OPML/HTML import/export are the operations that live here.

The `native/` module is the home for this pattern: anything the app shell triggers that carries ambient authority belongs there, never in `commands/`.

## The trap this guards against

The pull to resist is routing every operation through the JS/webview layer _for consistency_. File import/export is where that pull is strongest — the natural instinct is to hand it to JS because every other operation goes through JS — and the uniformity hides that a trust boundary runs through the middle of it. Consistency across operations is not a virtue at a security boundary. Before registering a new command, ask: does any argument carry authority (a path, a URL to be written to disk as-is, a raw identifier the backend will trust)? If yes, the orchestration belongs in Rust (`src-tauri/src/native/`), not in the webview — however inconsistent that looks next to the other commands.
