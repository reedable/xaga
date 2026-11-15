//! Production menu composition and theme helpers with isolated native windows.
#![allow(dead_code, unused_imports)]
include!("../src/lib.rs");
#[path = "support/app_state.rs"]
mod app_state;

fn main() {
    let directory = tempfile::tempdir().unwrap();
    let mut context = app_context();
    context.config_mut().app.windows.clear();
    let app = tauri::Builder::default()
        .manage(app_state::test_state(directory.path().to_path_buf()))
        .menu(menus::build)
        .on_menu_event(menus::on_event)
        .setup(|app| {
            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External("about:blank".parse()?),
            )
            .build()?;
            let seen = Arc::new(Mutex::new(Vec::new()));
            let captured = seen.clone();
            use tauri::Listener;
            app.listen("menu:set-theme", move |event| {
                captured.lock().unwrap().push(event.payload().to_owned());
            });
            for selected in ["light", "dark", "auto"] {
                let id = format!("theme_{selected}");
                menus::on_event(app.handle(), tauri::menu::MenuEvent { id: id.into() });
                let MenuItemKind::Submenu(view) = app.menu().unwrap().get("view_menu").unwrap()
                else {
                    panic!("View menu");
                };
                for theme in ["light", "dark", "auto"] {
                    let MenuItemKind::Check(item) =
                        view.get(format!("theme_{theme}").as_str()).unwrap()
                    else {
                        panic!("theme item");
                    };
                    assert_eq!(item.is_checked()?, theme == selected);
                    assert!(item.is_enabled()?);
                }
                assert_eq!(
                    seen.lock().unwrap().last().unwrap(),
                    &serde_json::to_string(selected)?
                );
                let color = theme_background_color(selected, &window);
                window.set_background_color(Some(color))?;
                let other = WebviewWindowBuilder::new(
                    app,
                    format!("fixture-{selected}"),
                    WebviewUrl::External("about:blank".parse()?),
                )
                .initialization_script(theme_bootstrap_script(selected))
                .build()?;
                other.set_background_color(Some(theme_background_color(selected, &other)))?;
                if selected != "auto" {
                    assert_eq!(theme_background_color(selected, &other), color);
                }
                other.close()?;
            }
            for writable in [false, true] {
                library_menu::update_imports(app.handle(), writable)?;
            }
            println!(
                "PASS: native menu theme routing, checkmarks, and existing/new window theme helpers"
            );
            app.handle().exit(0);
            Ok(())
        })
        .build(context)
        .unwrap();
    app.run(|_, _| {});
}
