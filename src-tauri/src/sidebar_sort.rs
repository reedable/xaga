use serde::{Deserialize, Serialize};
use tauri::{
    Runtime,
    menu::{CheckMenuItem, MenuItemKind},
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SidebarSort {
    #[default]
    Unread,
    Alphabetical,
}

pub(crate) fn selection(id: &str) -> Option<SidebarSort> {
    match id {
        "sidebar_sort_unread" => Some(SidebarSort::Unread),
        "sidebar_sort_name" => Some(SidebarSort::Alphabetical),
        _ => None,
    }
}

pub(crate) fn build<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<[CheckMenuItem<R>; 2]> {
    Ok([
        CheckMenuItem::with_id(
            app,
            "sidebar_sort_unread",
            "Sort by unread",
            true,
            true,
            None::<&str>,
        )?,
        CheckMenuItem::with_id(
            app,
            "sidebar_sort_name",
            "Sort by name",
            true,
            false,
            None::<&str>,
        )?,
    ])
}

pub(crate) fn update<R: Runtime>(app: &tauri::AppHandle<R>, selected: SidebarSort) {
    let Some(menu) = app.menu() else { return };
    let Some(MenuItemKind::Submenu(view)) = menu.get("view_menu") else {
        return;
    };
    for id in ["sidebar_sort_unread", "sidebar_sort_name"] {
        if let Some(MenuItemKind::Check(item)) = view.get(id) {
            item.set_checked(selection(id) == Some(selected)).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_and_serialized_values() {
        assert_eq!(SidebarSort::default(), SidebarSort::Unread);
        assert_eq!(
            serde_json::to_value(SidebarSort::Alphabetical).unwrap(),
            "alphabetical"
        );
        assert!(serde_json::from_str::<SidebarSort>("\"invalid\"").is_err());
        assert_eq!(
            selection("sidebar_sort_name"),
            Some(SidebarSort::Alphabetical)
        );
        assert_eq!(selection("show_sidebar"), None);
    }
}
