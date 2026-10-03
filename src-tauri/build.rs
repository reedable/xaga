fn main() {
    use std::{collections::BTreeSet, fs, path::PathBuf};
    println!("cargo:rerun-if-changed=command_registry.json");
    let registry: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("command_registry.json").unwrap()).unwrap();
    assert_eq!(
        registry.as_object().unwrap().len(),
        2,
        "unexpected command group"
    );
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let mut names = BTreeSet::new();
    let mut paths = Vec::new();
    let mut permissions = Vec::new();
    let mut constants = String::new();
    for (group, constant, permission) in [
        ("loader", "LOADER_COMMANDS", "allow-loader-callbacks"),
        ("trusted", "TRUSTED_COMMANDS", "allow-trusted-ui-commands"),
    ] {
        let commands: Vec<_> = registry[group]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| {
                let path = value.as_str().unwrap();
                assert!(path.split("::").all(|part| !part.is_empty()
                    && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')));
                let name = path.rsplit("::").next().unwrap();
                assert!(names.insert(name), "duplicate command: {name}");
                paths.push(path);
                name
            })
            .collect();
        constants.push_str(&format!("pub const {constant}: &[&str] = &{commands:?};\n"));
        permissions.push(serde_json::json!({
            "identifier": permission,
            "description": format!("Generated {group} commands from command_registry.json."),
            "commands": { "allow": commands }
        }));
    }
    fs::write(out.join("command_policy.rs"), constants).unwrap();
    fs::write(
        out.join("command_handler.rs"),
        format!("tauri::generate_handler![{}]", paths.join(",")),
    )
    .unwrap();
    let permission_path = out.join("application-permissions.json");
    fs::write(
        &permission_path,
        serde_json::to_vec_pretty(&serde_json::json!({"permission": permissions})).unwrap(),
    )
    .unwrap();
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().permissions_path_pattern(Box::leak(
                permission_path
                    .to_string_lossy()
                    .into_owned()
                    .into_boxed_str(),
            )),
        ),
    )
    .expect("build application capabilities");
}
