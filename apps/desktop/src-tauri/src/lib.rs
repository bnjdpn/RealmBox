mod ai;
mod launcher;
pub mod local_guide;
mod runtime_instance;
mod setup;
mod solo_profile_store;
pub mod solo_profiles;

use std::{
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use ai::AiCapability;
use launcher::{
    BotPresence, ClientChoice, DialogueChattiness, ErrorCode, GameDataInspection,
    InstallationOptions, LauncherPhase, LauncherProgress, LauncherService, LauncherStatus,
    OperationComponent, OperationStep, RealmBackupSummary, RealmDiagnostics, SystemCommandRunner,
};
use local_guide::{LocalGuideQuery, LocalGuideResponse};
use runtime_instance::RuntimeInstanceGuard;
use solo_profile_store::SoloProfileView;
use solo_profiles::SoloProfile;
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState(Arc<Mutex<LauncherService<SystemCommandRunner>>>);

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallRealmRequest {
    game_data_path: String,
    client_choice: ClientChoice,
    bots_enabled: bool,
    bot_count: usize,
    bot_presence: BotPresence,
    ai_enabled: bool,
    ai_model: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum RecoveryAction {
    Retry,
    ChooseGameData,
    StartDocker,
    OpenDiagnostics,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LauncherCommandError {
    code: ErrorCode,
    component: &'static str,
    technical_detail: Option<String>,
    recovery_actions: Vec<RecoveryAction>,
}

impl LauncherCommandError {
    fn new(detail: impl Into<String>, fallback: ErrorCode, component: &'static str) -> Self {
        let detail = detail.into();
        let code = ErrorCode::from_detail(&detail, fallback);
        let recovery_actions = match code {
            ErrorCode::DockerMissing | ErrorCode::DockerNotRunning => {
                vec![RecoveryAction::StartDocker, RecoveryAction::Retry]
            }
            ErrorCode::GameDataIncomplete | ErrorCode::GameBuildUnsupported => {
                vec![RecoveryAction::ChooseGameData, RecoveryAction::Retry]
            }
            _ => vec![RecoveryAction::Retry, RecoveryAction::OpenDiagnostics],
        };
        Self {
            code,
            component,
            technical_detail: Some(detail),
            recovery_actions,
        }
    }
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
                                    operation_id: format!("client-monitor-{process_id}"),
                                    component: OperationComponent::Client,
                                    step: OperationStep::Stop,
                                    phase: LauncherPhase::Error,
                                    message: "Arrêt automatique incomplet".into(),
                                    detail: Some(error),
                                    error_code: Some(ErrorCode::OperationUnavailable),
                                    progress: 0,
                                    completed_bytes: None,
                                    total_bytes: None,
                                    cancellable: false,
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
                            operation_id: format!("client-monitor-{process_id}"),
                            component: OperationComponent::Client,
                            step: OperationStep::Validate,
                            phase: LauncherPhase::Error,
                            message: "Surveillance du client interrompue".into(),
                            detail: Some(error),
                            error_code: Some(ErrorCode::OperationUnavailable),
                            progress: 0,
                            completed_bytes: None,
                            total_bytes: None,
                            cancellable: false,
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
) -> Result<LauncherStatus, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .bootstrap(|progress| emit_progress(&app_handle, progress))
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(
            error.to_string(),
            ErrorCode::OperationUnavailable,
            "launcher",
        )
    })?
    .map_err(|error| {
        LauncherCommandError::new(error, ErrorCode::InstallationStateUnreadable, "launcher")
    })?;
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
) -> Result<LauncherStatus, LauncherCommandError> {
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
                    bot_presence: request.bot_presence,
                    ai_enabled: request.ai_enabled,
                    ai_model: request.ai_model,
                },
                |progress| emit_progress(&app_handle, progress),
            )
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(
            error.to_string(),
            ErrorCode::OperationUnavailable,
            "launcher",
        )
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::InstallationIncomplete, "server"))
}

#[tauri::command]
async fn start_realm(
    app: AppHandle,
    state: State<'_, AppState>,
    bots_enabled: bool,
    bot_count: usize,
    bot_presence: BotPresence,
    ai_enabled: bool,
) -> Result<LauncherStatus, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .start(
                Some(bots_enabled),
                Some(bot_count),
                Some(bot_presence),
                Some(ai_enabled),
                |progress| emit_progress(&app_handle, progress),
            )
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(
            error.to_string(),
            ErrorCode::OperationUnavailable,
            "launcher",
        )
    })?
    .map_err(|error| {
        LauncherCommandError::new(error, ErrorCode::InstallationIncomplete, "server")
    })?;
    if status.phase == LauncherPhase::Running {
        monitor_client(app, Arc::clone(&state.0));
    }
    Ok(status)
}

#[tauri::command]
async fn inspect_ai_capability(
    state: State<'_, AppState>,
) -> Result<AiCapability, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        Ok::<AiCapability, String>(
            service
                .lock()
                .map_err(|_| "état du lanceur indisponible".to_string())?
                .inspect_ai_capability(),
        )
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(error.to_string(), ErrorCode::OperationUnavailable, "ai")
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "ai"))
}

#[tauri::command]
async fn configure_local_dialogue(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
    model: Option<String>,
) -> Result<LauncherStatus, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .configure_local_dialogue(enabled, model, |progress| {
                emit_progress(&app_handle, progress)
            })
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(error.to_string(), ErrorCode::OperationUnavailable, "ai")
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "ai"))
}

#[tauri::command]
async fn configure_dialogue_chattiness(
    state: State<'_, AppState>,
    chattiness: DialogueChattiness,
) -> Result<LauncherStatus, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .configure_dialogue_chattiness(chattiness)
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(error.to_string(), ErrorCode::OperationUnavailable, "ai")
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "ai"))
}

#[tauri::command]
async fn inspect_installation(
    state: State<'_, AppState>,
    model: Option<String>,
) -> Result<setup::InstallationCheck, String> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .inspect_installation(model.as_deref())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn open_setup_resource(resource: setup::SetupResource) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        setup::open_resource(&SystemCommandRunner::default(), resource)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn inspect_game_data(
    game_data_path: String,
) -> Result<GameDataInspection, LauncherCommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        launcher::inspect_game_data_root(Path::new(&game_data_path))
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(error.to_string(), ErrorCode::OperationUnavailable, "client")
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::GameDataIncomplete, "client"))
}

#[tauri::command]
async fn change_game_data_path(
    state: State<'_, AppState>,
    game_data_path: String,
) -> Result<LauncherStatus, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .change_game_data_path(Path::new(&game_data_path))
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(error.to_string(), ErrorCode::OperationUnavailable, "client")
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::GameDataIncomplete, "client"))
}

#[tauri::command]
async fn stop_realm(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LauncherStatus, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .stop(|progress| emit_progress(&app_handle, progress))
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(
            error.to_string(),
            ErrorCode::OperationUnavailable,
            "launcher",
        )
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "launcher"))
}

#[tauri::command]
async fn restore_last_recovery(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LauncherStatus, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .restore_last_recovery(|progress| emit_progress(&app_handle, progress))
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(
            error.to_string(),
            ErrorCode::OperationUnavailable,
            "launcher",
        )
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::RecoveryFailed, "database"))
}

#[tauri::command]
async fn update_playerbot_population(
    state: State<'_, AppState>,
    bots_enabled: bool,
    bot_count: usize,
    bot_presence: BotPresence,
) -> Result<LauncherStatus, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .update_playerbot_population(bots_enabled, bot_count, bot_presence)
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(error.to_string(), ErrorCode::OperationUnavailable, "bots")
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "bots"))
}

#[tauri::command]
async fn inspect_realm_backup(
    state: State<'_, AppState>,
) -> Result<Option<RealmBackupSummary>, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .inspect_realm_backup()
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(
            error.to_string(),
            ErrorCode::OperationUnavailable,
            "database",
        )
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::BackupFailed, "database"))
}

#[tauri::command]
async fn create_realm_backup(
    state: State<'_, AppState>,
) -> Result<RealmBackupSummary, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .create_realm_backup()
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(
            error.to_string(),
            ErrorCode::OperationUnavailable,
            "database",
        )
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::BackupFailed, "database"))
}

#[tauri::command]
async fn query_local_guide(
    state: State<'_, AppState>,
    query: LocalGuideQuery,
) -> Result<LocalGuideResponse, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .query_local_guide(query)
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(
            error.to_string(),
            ErrorCode::OperationUnavailable,
            "database",
        )
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "database"))
}

#[tauri::command]
async fn inspect_solo_profiles(
    state: State<'_, AppState>,
) -> Result<SoloProfileView, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .inspect_solo_profiles()
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(error.to_string(), ErrorCode::OperationUnavailable, "server")
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "server"))
}

#[tauri::command]
async fn configure_solo_profile(
    state: State<'_, AppState>,
    profile: SoloProfile,
) -> Result<SoloProfileView, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .configure_solo_profile(profile)
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(error.to_string(), ErrorCode::OperationUnavailable, "server")
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "server"))
}

#[tauri::command]
async fn rollback_solo_profile(
    state: State<'_, AppState>,
) -> Result<SoloProfileView, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .rollback_solo_profile()
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(error.to_string(), ErrorCode::OperationUnavailable, "server")
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "server"))
}

#[tauri::command]
async fn get_realm_diagnostics(
    state: State<'_, AppState>,
) -> Result<RealmDiagnostics, LauncherCommandError> {
    let service = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        service
            .lock()
            .map_err(|_| "état du lanceur indisponible".to_string())?
            .diagnostics()
    })
    .await
    .map_err(|error| {
        LauncherCommandError::new(
            error.to_string(),
            ErrorCode::OperationUnavailable,
            "launcher",
        )
    })?
    .map_err(|error| LauncherCommandError::new(error, ErrorCode::OperationUnavailable, "launcher"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let instance_guard = RuntimeInstanceGuard::acquire(&app_data)?;
            let resource_dir = app.path().resource_dir()?;
            let bundled_addon = resource_dir.join("addons/RealmBoxCompanions");
            let development_addon =
                std::env::current_dir()?.join("../../addons/RealmBoxCompanions");
            let addon_source = if bundled_addon.is_dir() {
                bundled_addon
            } else {
                development_addon
            };
            let service =
                LauncherService::new(app_data, addon_source, SystemCommandRunner::default())?;
            app.manage(instance_guard);
            app.manage(AppState(Arc::new(Mutex::new(service))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_launcher,
            inspect_installation,
            open_setup_resource,
            install_realm,
            start_realm,
            stop_realm,
            restore_last_recovery,
            update_playerbot_population,
            inspect_realm_backup,
            create_realm_backup,
            query_local_guide,
            inspect_solo_profiles,
            configure_solo_profile,
            rollback_solo_profile,
            get_realm_diagnostics,
            inspect_ai_capability,
            configure_local_dialogue,
            configure_dialogue_chattiness,
            inspect_game_data,
            change_game_data_path
        ])
        .run(tauri::generate_context!())
        .expect("RealmBox n'a pas pu initialiser sa boucle applicative");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_errors_expose_stable_codes_and_recovery_actions() {
        let error = LauncherCommandError::new(
            "Docker Desktop doit être démarré: moteur indisponible",
            ErrorCode::OperationUnavailable,
            "launcher",
        );
        let value = serde_json::to_value(error).expect("serialized error");
        assert_eq!(value["code"], "dockerNotRunning");
        assert_eq!(value["component"], "launcher");
        assert_eq!(value["recoveryActions"][0], "startDocker");

        let server_error = LauncherCommandError::new(
            "docker a échoué; consultez start-server-data.log",
            ErrorCode::InstallationIncomplete,
            "server",
        );
        let server_value = serde_json::to_value(server_error).expect("serialized server error");
        assert_eq!(server_value["code"], "installationIncomplete");
        assert_eq!(server_value["component"], "server");
    }
}
