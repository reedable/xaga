#[path = "../src/library_menu.rs"]
mod library_menu;
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() {
    use tauri::menu::{Menu, MenuItem};
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app = tauri::Builder::default()
        .setup(|app| {
            let ids = [
                "import_feeds",
                "import_bookmarks",
                "export_feeds",
                "export_bookmarks",
                "open_library",
            ];
            let controls: Vec<_> = ids
                .iter()
                .map(|id| MenuItem::with_id(app, *id, *id, true, None::<&str>).unwrap())
                .collect();
            let refs: Vec<&dyn tauri::menu::IsMenuItem<_>> =
                controls.iter().map(|control| control as _).collect();
            app.set_menu(Menu::with_items(app, &refs)?)?;
            for writable in [false, true, false] {
                library_menu::update_imports(app.handle(), writable)?;
                for (index, control) in controls.iter().enumerate() {
                    assert_eq!(control.is_enabled()?, index >= 2 || writable);
                }
            }
            println!(
                "PASS: native imports follow library mode; export and switching remain enabled"
            );
            app.handle().exit(0);
            Ok(())
        })
        .build(context)
        .unwrap();
    app.run(|_, _| {});
}
