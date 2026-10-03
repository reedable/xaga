//! Exercise Tauri's actual ACL before the application invoke gate.
use tauri::{WebviewUrl, WebviewWindowBuilder, ipc::CallbackFn, webview::InvokeRequest};

#[test]
fn permissions_cover_registered_commands_without_overlap() {
    use std::collections::BTreeSet;
    let source = include_str!("../src/lib.rs");
    let handler = source
        .split("tauri::generate_handler![")
        .nth(1)
        .unwrap()
        .split("]);")
        .next()
        .unwrap();
    let registered: BTreeSet<_> = handler
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("commands::"))
        .map(|line| line.trim_end_matches(',').rsplit("::").next().unwrap())
        .collect();
    let permissions: serde_json::Value =
        serde_json::from_str(include_str!("../permissions/application.json")).unwrap();
    let mut allowed = BTreeSet::new();
    for permission in permissions["permission"].as_array().unwrap() {
        for command in permission["commands"]["allow"].as_array().unwrap() {
            assert!(
                allowed.insert(command.as_str().unwrap()),
                "command appears in both permission groups"
            );
        }
    }
    assert_eq!(
        registered, allowed,
        "update application permissions when registering commands"
    );
}

#[test]
fn loader_callbacks_are_allowed_only_in_remote_loader_windows() {
    let app = tauri::test::mock_builder()
        .invoke_handler(|invoke| {
            invoke.resolver.resolve("reached application gate");
            true
        })
        .build(tauri::generate_context!())
        .unwrap();

    for label in ["loader-7", "main", "settings", "unknown"] {
        let window = WebviewWindowBuilder::new(&app, label, WebviewUrl::default())
            .build()
            .unwrap();
        for origin in [
            "https://example.org/article",
            "http://example.org/article",
            "tauri://localhost",
        ] {
            for command in [
                "add_loader_diagnostic_event",
                "receive_html",
                "receive_favicon",
                "loader_will_unload",
                "loader_page_loaded",
                "capture_and_close_loader",
                "read_article",
                "set_library",
            ] {
                let response = tauri::test::get_ipc_response(
                    &window,
                    InvokeRequest {
                        cmd: command.into(),
                        callback: CallbackFn(0),
                        error: CallbackFn(1),
                        url: origin.parse().unwrap(),
                        body: serde_json::json!({}).into(),
                        headers: Default::default(),
                        invoke_key: tauri::test::INVOKE_KEY.into(),
                    },
                );
                let loader_command = !matches!(command, "read_article" | "set_library");
                let expected = if origin.starts_with("tauri:") {
                    matches!(label, "main" | "settings") && !loader_command
                } else {
                    label == "loader-7" && loader_command
                };
                assert_eq!(
                    response.is_ok(),
                    expected,
                    "{label} {origin} {command}: {response:?}"
                );
            }
        }
    }
}
