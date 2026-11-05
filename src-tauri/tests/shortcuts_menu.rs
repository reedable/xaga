//! Real macOS menu construction, replacement, window gating, and native action dispatch.
//! Run in a desktop session; this does not open a user's library.
#[path = "../src/error.rs"]
#[allow(dead_code, unused_imports)]
mod error;
#[path = "../src/shortcuts_menu.rs"]
mod shortcuts_menu;

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::{MainThreadMarker, msg_send, runtime::AnyObject};
    use objc2_app_kit::NSApplication;
    use std::sync::{Arc, Mutex};
    use tauri::{
        RunEvent, WebviewUrl, WebviewWindowBuilder,
        menu::{Menu, MenuItemKind, Submenu},
    };

    let received = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured = received.clone();
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app = tauri::Builder::default()
        .on_menu_event(move |app, event| {
            if let Some(key) = shortcuts_menu::selection(event.id().as_ref()) {
                let mut received = captured.lock().unwrap();
                received.push(key.to_string());
                if received.len() == 2 {
                    assert_eq!(*received, ["all", "latest"]);
                    app.exit(0);
                }
            }
        })
        .setup(|app| {
            let history = Submenu::with_id(app, "history", "History", true)?;
            let shortcuts = shortcuts_menu::build(app.handle(), &[])?;
            let window_menu = Submenu::with_id(app, "window", "Window", true)?;
            let menu = Menu::with_items(app, &[&history, &shortcuts, &window_menu])?;
            app.set_menu(menu)?;
            let main = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External("about:blank".parse()?),
            )
            .visible(false)
            .build()?;
            let settings = WebviewWindowBuilder::new(
                app,
                "settings",
                WebviewUrl::External("about:blank".parse()?),
            )
            .visible(false)
            .build()?;
            let items = || {
                serde_json::from_value(serde_json::json!([
                    {"key": "latest", "title": "Renamed target", "enabled": true},
                    {"key": "missing", "title": "Shortcut unavailable", "enabled": true},
                    {"key": "loading", "title": "Loading shortcut…", "enabled": false}
                ]))
                .unwrap()
            };
            assert!(matches!(
                shortcuts_menu::set_shortcuts_menu(app.handle().clone(), settings, items()),
                Err(error::AppError::Unauthorized)
            ));
            assert_eq!(shortcuts.items()?.len(), 1);
            shortcuts_menu::set_shortcuts_menu(app.handle().clone(), main.clone(), items())?;
            let menu = app.menu().unwrap();
            assert_eq!(
                menu.items()?
                    .iter()
                    .map(|item| item.id().as_ref())
                    .collect::<Vec<_>>(),
                ["history", "shortcuts_menu", "window"]
            );
            let MenuItemKind::Submenu(projected) = menu.items()?.remove(1) else {
                panic!("submenu")
            };
            let rows = projected.items()?;
            assert_eq!(rows.len(), 5);
            let MenuItemKind::MenuItem(target) = &rows[2] else {
                panic!("target")
            };
            assert_eq!(target.text()?, "Renamed target");
            let MenuItemKind::MenuItem(loading) = &rows[4] else {
                panic!("loading")
            };
            assert!(!loading.is_enabled()?);
            // Empty projection removes the separator and targets, preserving All items.
            shortcuts_menu::set_shortcuts_menu(app.handle().clone(), main.clone(), Vec::new())?;
            let MenuItemKind::Submenu(empty) = app.menu().unwrap().items()?.remove(1) else {
                panic!("empty")
            };
            assert_eq!(empty.items()?.len(), 1);
            shortcuts_menu::set_shortcuts_menu(app.handle().clone(), main, items())?;
            Ok(())
        })
        .build(context)
        .expect("native menu test app");
    let timeout = app.handle().clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(15));
        eprintln!("native shortcut action dispatch timed out");
        timeout.exit(1);
    });
    app.run(move |_, event| {
        if matches!(event, RunEvent::Ready) {
            let mtm = MainThreadMarker::new().unwrap();
            let native_app = NSApplication::sharedApplication(mtm);
            // SAFETY: live NSApplication menu objects, accessed on the main thread.
            // Trigger actual Cocoa menu actions rather than calling our handler.
            unsafe {
                let menu: *mut AnyObject = msg_send![&*native_app, mainMenu];
                let item: *mut AnyObject = msg_send![menu, itemAtIndex: 1isize];
                let submenu: *mut AnyObject = msg_send![item, submenu];
                let _: () = msg_send![submenu, performActionForItemAtIndex: 0isize];
                let _: () = msg_send![submenu, performActionForItemAtIndex: 2isize];
            }
        }
        if matches!(event, RunEvent::Exit) {
            assert_eq!(*received.lock().unwrap(), ["all", "latest"]);
            eprintln!("native shortcuts menu passed");
        }
    });
}
