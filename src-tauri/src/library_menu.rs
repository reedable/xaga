use tauri::{Runtime, menu::MenuItemKind};

/// Imports are library writes; exports and library switching are not.
pub fn update_imports<R: Runtime>(app: &tauri::AppHandle<R>, writable: bool) -> tauri::Result<()> {
    for id in ["import_feeds", "import_bookmarks"] {
        if let Some(MenuItemKind::MenuItem(control)) = app.menu().and_then(|menu| menu.get(id)) {
            control.set_enabled(writable)?;
        }
    }
    Ok(())
}
