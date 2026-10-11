//! Exercise the generated ACL both alone and with the production native gate.
#[path = "../src/command_policy.rs"]
mod command_policy;

use tauri::{WebviewUrl, WebviewWindowBuilder, ipc::CallbackFn, webview::InvokeRequest};

fn check_matrix(with_native_gate: bool) {
    let app = tauri::test::mock_builder()
        .invoke_handler(move |invoke| {
            if with_native_gate
                && !command_policy::is_invoke_allowed(
                    invoke.message.webview_ref().label(),
                    invoke.message.command(),
                )
            {
                invoke.resolver.reject("native gate denied");
            } else {
                invoke.resolver.resolve("accepted");
            }
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
            "file:///tmp/page.html",
            "data:text/html,test",
        ] {
            for command in command_policy::LOADER_COMMANDS
                .iter()
                .chain(command_policy::TRUSTED_COMMANDS)
                .chain([&"unregistered_command"])
            {
                let response = tauri::test::get_ipc_response(
                    &window,
                    InvokeRequest {
                        cmd: (*command).into(),
                        callback: CallbackFn(0),
                        error: CallbackFn(1),
                        url: origin.parse().unwrap(),
                        body: serde_json::json!({}).into(),
                        headers: Default::default(),
                        invoke_key: tauri::test::INVOKE_KEY.into(),
                    },
                );
                let expected = match origin {
                    "tauri://localhost" => {
                        matches!(label, "main" | "settings")
                            && command_policy::TRUSTED_COMMANDS.contains(command)
                    }
                    origin if origin.starts_with("http:") || origin.starts_with("https:") => {
                        label == "loader-7" && command_policy::LOADER_COMMANDS.contains(command)
                    }
                    _ => false,
                };
                assert_eq!(
                    response.is_ok(),
                    expected,
                    "gate={with_native_gate} {label} {origin} {command}: {response:?}"
                );
            }
        }
    }
}

#[test]
fn tauri_acl_enforces_every_registered_command() {
    check_matrix(false);
}

#[test]
fn tauri_acl_and_production_gate_agree() {
    check_matrix(true);
}

#[test]
fn loader_authority_remains_the_reviewed_six_callbacks() {
    // Intentional security contract, independent of the generated classification.
    let mut actual = command_policy::LOADER_COMMANDS.to_vec();
    let mut expected = vec![
        "add_loader_diagnostic_event",
        "receive_html",
        "receive_favicon",
        "loader_will_unload",
        "loader_page_loaded",
        "capture_and_close_loader",
    ];
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    assert!(command_policy::TRUSTED_COMMANDS.contains(&"set_find_enabled"));
    for label in ["main", "settings", "loader-7", "unknown"] {
        assert!(!command_policy::is_invoke_allowed(
            label,
            "unregistered_command"
        ));
    }
}
