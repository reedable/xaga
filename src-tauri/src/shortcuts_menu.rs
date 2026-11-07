//! Native projection of the UI's resolved shortcuts. No library reads or writes.
use serde::Deserialize;
use tauri::{
    Runtime,
    menu::{MenuItem, MenuItemKind, PredefinedMenuItem, Submenu},
};

use crate::error::AppError;

const MENU_ID: &str = "shortcuts_menu";
const ITEM_PREFIX: &str = "shortcut:";

#[derive(Deserialize)]
pub(crate) struct ShortcutMenuItem {
    key: String,
    title: String,
    enabled: bool,
    children: Option<Vec<ShortcutMenuItem>>,
}

pub(crate) fn build<R: Runtime>(
    app: &tauri::AppHandle<R>,
    items: &[ShortcutMenuItem],
) -> tauri::Result<Submenu<R>> {
    let menu = Submenu::with_id(app, MENU_ID, "Shortcuts", true)?;
    menu.append(&MenuItem::with_id(
        app,
        "shortcut:all",
        "All items",
        true,
        None::<&str>,
    )?)?;
    if !items.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    append_items(app, &menu, items)?;
    Ok(menu)
}

fn append_items<R: Runtime>(
    app: &tauri::AppHandle<R>,
    menu: &Submenu<R>,
    items: &[ShortcutMenuItem],
) -> tauri::Result<()> {
    for item in items {
        if let Some(children) = &item.children {
            let submenu = Submenu::with_id(app, format!("group:{}", item.key), &item.title, true)?;
            if children.is_empty() {
                submenu.append(&MenuItem::with_id(
                    app,
                    format!("empty:{}", item.key),
                    "Empty",
                    false,
                    None::<&str>,
                )?)?;
            } else {
                append_items(app, &submenu, children)?;
            }
            menu.append(&submenu)?;
        } else {
            menu.append(&MenuItem::with_id(
                app,
                format!("{ITEM_PREFIX}{}", item.key),
                &item.title,
                item.enabled,
                None::<&str>,
            )?)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn set_shortcuts_menu(
    app: tauri::AppHandle,
    webview: tauri::WebviewWindow,
    items: Vec<ShortcutMenuItem>,
) -> Result<(), AppError> {
    // The settings window shares application commands, but cannot replace the
    // main window's navigation projection.
    if webview.label() != "main" {
        return Err(AppError::Unauthorized);
    }
    replace(&app, &items).map_err(|error| AppError::Message(error.to_string()))
}

fn replace<R: Runtime>(app: &tauri::AppHandle<R>, items: &[ShortcutMenuItem]) -> tauri::Result<()> {
    let Some(menu) = app.menu() else {
        return Ok(());
    };
    let Some(position) = menu.items()?.iter().position(
        |item| matches!(item, MenuItemKind::Submenu(submenu) if submenu.id().as_ref() == MENU_ID),
    ) else {
        return Ok(());
    };
    let replacement = build(app, items)?;
    // Construct first, so a construction failure preserves the current menu.
    menu.remove_at(position)?;
    menu.insert(&replacement, position)?;
    Ok(())
}

pub(crate) fn selection(id: &str) -> Option<&str> {
    id.strip_prefix(ITEM_PREFIX)
}
