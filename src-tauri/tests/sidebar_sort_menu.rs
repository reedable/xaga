//! Native sort menu construction, checkmarks, and Cocoa action dispatch.
#[path = "../src/sidebar_sort.rs"]
#[allow(dead_code, unused_imports)]
mod sidebar_sort;

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::{MainThreadMarker, msg_send, runtime::AnyObject};
    use objc2_app_kit::NSApplication;
    use sidebar_sort::SidebarSort;
    use std::sync::{Arc, Mutex};
    use tauri::{
        RunEvent,
        menu::{CheckMenuItem, Menu, MenuItemKind, Submenu},
    };
    let received = Arc::new(Mutex::new(Vec::new()));
    let captured = received.clone();
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app = tauri::Builder::default()
        .on_menu_event(move |app, event| {
            if let Some(value) = sidebar_sort::selection(event.id().as_ref()) {
                sidebar_sort::update(app, value);
                let MenuItemKind::Submenu(view) = app.menu().unwrap().get("view_menu").unwrap()
                else {
                    panic!("View menu missing");
                };
                for id in ["sidebar_sort_unread", "sidebar_sort_name"] {
                    let MenuItemKind::Check(item) = view.get(id).unwrap() else {
                        panic!("check item missing");
                    };
                    assert_eq!(
                        item.is_checked().unwrap(),
                        sidebar_sort::selection(id) == Some(value)
                    );
                    assert!(item.is_enabled().unwrap());
                }
                let mut values = captured.lock().unwrap();
                values.push(value);
                if values.len() == 2 {
                    app.exit(0);
                }
            }
        })
        .setup(|app| {
            let sort = sidebar_sort::build(app.handle())?;
            assert!(sort[0].is_checked()?);
            assert!(!sort[1].is_checked()?);
            assert_eq!(sort[0].id().as_ref(), "sidebar_sort_unread");
            assert_eq!(sort[1].id().as_ref(), "sidebar_sort_name");
            let show = CheckMenuItem::with_id(
                app,
                "show_sidebar",
                "Show Sidebar",
                true,
                true,
                None::<&str>,
            )?;
            let view = Submenu::with_id_and_items(
                app,
                "view_menu",
                "View",
                true,
                &[&show, &sort[0], &sort[1]],
            )?;
            app.set_menu(Menu::with_items(app, &[&view])?)?;
            // Same update operation used by successful UI preference writes and startup restore.
            sidebar_sort::update(app.handle(), SidebarSort::Alphabetical);
            assert!(!sort[0].is_checked()?);
            assert!(sort[1].is_checked()?);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                handle.exit(1);
            });
            Ok(())
        })
        .build(context)
        .unwrap();
    app.run(move |_, event| {
        if matches!(event, RunEvent::Ready) {
            let mtm = MainThreadMarker::new().unwrap();
            let native = NSApplication::sharedApplication(mtm);
            // SAFETY: live Cocoa menu objects, accessed on the main thread.
            unsafe {
                let menu: *mut AnyObject = msg_send![&*native, mainMenu];
                let item: *mut AnyObject = msg_send![menu, itemAtIndex: 0isize];
                let view: *mut AnyObject = msg_send![item, submenu];
                let _: () = msg_send![view, performActionForItemAtIndex: 1isize];
                let _: () = msg_send![view, performActionForItemAtIndex: 2isize];
            }
        }
        if matches!(event, RunEvent::Exit) {
            assert_eq!(
                *received.lock().unwrap(),
                [SidebarSort::Unread, SidebarSort::Alphabetical]
            );
            println!("PASS: native sidebar sort menu synchronization and dispatch");
        }
    });
}
