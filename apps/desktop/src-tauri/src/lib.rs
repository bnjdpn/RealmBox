mod ai;
mod launcher;

use std::{
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use ai::AiCapability;
use launcher::{
    ClientChoice, GameDataInspection, InstallationOptions, LauncherPhase, LauncherProgress,
    LauncherService, LauncherStatus, SystemCommandRunner,
};
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState(Arc<Mutex<LauncherService<SystemCommandRunner>>>);

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallRealmRequest {
    game_data_path: String,
    client_choice: ClientChoice,
    bots_enabled: bool,
    bot_count: usize,
    ai_enabled: bool,
    ai_model: Option<String>,
}

fn emit_progress(app: &AppHandle, progress: LauncherProgress) {
    let _ = app.emit("realmbox://progress", progress);
}

fn monitor_client(app: AppHandle, service: Arc<Mutex<LauncherService<SystemCommandRunner>>>) {
    let process_id = service
        .lock()
        .ok()
        .and_then(|launcher| launcher.client_process_id());
    let Some(process_id) = process_id else {
        return;
    };
    tauri::async_runtime::spawn_blocking(move || {
        loop {
            thread::sleep(Duration::from_secs(2));
            let running = service
                .lock()
                .map_err(|_| "état du lanceur indisponible".to_string())
                .and_then(|launcher| launcher.is_client_process_running(process_id));
            match running {
                Ok(true) => continue,
                Ok(false) => {
                    let result = service
                        .lock()
                        .map_err(|_| "état du lanceur indisponible".to_string())
                        .and_then(|mut launcher| {
                            launcher.stop_after_client_exit(process_id, |progress| {
                                emit_progress(&app, progress)
                            })
                        });
                    match result {
                        Ok(Some(status)) => {
                            let _ = app.emit("realmbox://status", status);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            emit_progress(
                                &app,
                                LauncherProgress {
                                    phase: LauncherPhase::Error,
                                    message: "Arrêt automatique incomplet".into(),
                                    detail: Some(error),
                                    progress: 0,
                                },
                            );
                        }
                    }
                    break;
                }
                Err(error) => {
                    emit_progress(
                        &app,
                        LauncherProgress {
                            phase: LauncherPhase::Error,
                            message: "Surveillance du client interrompue".into(),
                            detail: Some(error),
                            progress: 0,
                        },
                    );
                    break;
                }
            }
        }
    });
}

#[tauri::command]
async fn bootstrap_launcher(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LauncherStatus, String> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .bootstrap(|progress| emit_progress(&app_handle, progress))
    })
    .await
    .map_err(|error| error.to_string())??;
    if status.phase == LauncherPhase::Running {
        monitor_client(app, Arc::clone(&state.0));
    }
    Ok(status)
}

#[tauri::command]
async fn install_realm(
    app: AppHandle,
    state: State<'_, AppState>,
    request: InstallRealmRequest,
) -> Result<LauncherStatus, String> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .install(
                request.game_data_path.as_ref(),
                InstallationOptions {
                    client_choice: request.client_choice,
                    bots_enabled: request.bots_enabled,
                    bot_count: request.bot_count,
                    ai_enabled: request.ai_enabled,
                    ai_model: request.ai_model,
                },
                |progress| emit_progress(&app_handle, progress),
            )
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn start_realm(
    app: AppHandle,
    state: State<'_, AppState>,
    bots_enabled: bool,
    bot_count: usize,
    ai_enabled: bool,
) -> Result<LauncherStatus, String> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .start(
                Some(bots_enabled),
                Some(bot_count),
                Some(ai_enabled),
                |progress| emit_progress(&app_handle, progress),
            )
    })
    .await
    .map_err(|error| error.to_string())??;
    if status.phase == LauncherPhase::Running {
        monitor_client(app, Arc::clone(&state.0));
    }
    Ok(status)
}

#[tauri::command]
async fn inspect_ai_capability(state: State<'_, AppState>) -> Result<AiCapability, String> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        Ok(service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .inspect_ai_capability())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn inspect_game_data(game_data_path: String) -> Result<GameDataInspection, String> {
    tauri::async_runtime::spawn_blocking(move || {
        launcher::inspect_game_data_root(Path::new(&game_data_path))
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
            stop_realm,
            inspect_ai_capability,
            inspect_game_data
        ])
        .run(tauri::generate_context!())
        .expect("RealmBox n'a pas pu initialiser sa boucle applicative");
}
