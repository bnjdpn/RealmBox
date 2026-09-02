mod launcher;

use std::sync::{Arc, Mutex};

use launcher::{LauncherProgress, LauncherService, LauncherStatus, SystemCommandRunner};
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState(Arc<Mutex<LauncherService<SystemCommandRunner>>>);

fn emit_progress(app: &AppHandle, progress: LauncherProgress) {
    let _ = app.emit("realmbox://progress", progress);
}

#[tauri::command]
async fn bootstrap_launcher(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LauncherStatus, String> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .bootstrap(|progress| emit_progress(&app_handle, progress))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn install_realm(
    app: AppHandle,
    state: State<'_, AppState>,
    game_data_path: String,
    bots_enabled: bool,
) -> Result<LauncherStatus, String> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .install(game_data_path.as_ref(), bots_enabled, |progress| {
                emit_progress(&app_handle, progress)
            })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn start_realm(
    app: AppHandle,
    state: State<'_, AppState>,
    bots_enabled: bool,
) -> Result<LauncherStatus, String> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .start(Some(bots_enabled), |progress| {
                emit_progress(&app_handle, progress)
            })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn stop_realm(app: AppHandle, state: State<'_, AppState>) -> Result<LauncherStatus, String> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .stop(|progress| emit_progress(&app_handle, progress))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let resource_dir = app.path().resource_dir()?;
            let bundled_addon = resource_dir.join("addons/RealmBoxCompanions");
            let development_addon =
                std::env::current_dir()?.join("../../addons/RealmBoxCompanions");
            let addon_source = if bundled_addon.is_dir() {
                bundled_addon
            } else {
                development_addon
            };
            let service = LauncherService::new(app_data, addon_source, SystemCommandRunner)?;
            app.manage(AppState(Arc::new(Mutex::new(service))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_launcher,
            install_realm,
            start_realm,
            stop_realm
        ])
        .run(tauri::generate_context!())
        .expect("RealmBox n'a pas pu initialiser sa boucle applicative");
}
