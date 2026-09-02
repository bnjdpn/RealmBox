fn main() {
    for variable in [
        "REALMBOX_AUTH_SERVER_IMAGE",
        "REALMBOX_WORLD_SERVER_IMAGE",
        "REALMBOX_DB_IMPORT_IMAGE",
        "REALMBOX_TOOLS_IMAGE",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    tauri_build::build()
}
