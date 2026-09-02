use std::sync::Mutex;

use realmbox_client::FakeClientBackend;
use realmbox_orchestrator::{Orchestrator, PlayerDashboard};
use realmbox_storage::StateStore;
use tauri::{Manager, State};

struct AppState(Mutex<Orchestrator<FakeClientBackend>>);

#[tauri::command]
fn prepare_fake_world(state: State<'_, AppState>) -> Result<PlayerDashboard, String> {
    let mut orchestrator = state
        .0
        .lock()
        .map_err(|_| "état applicatif indisponible".to_string())?;
    orchestrator
        .run_fake_setup()
        .map_err(|error| error.to_string())?;
    Ok(orchestrator.dashboard().clone())
}

#[tauri::command]
fn start_fake_world(state: State<'_, AppState>) -> Result<PlayerDashboard, String> {
    let mut orchestrator = state
        .0
        .lock()
        .map_err(|_| "état applicatif indisponible".to_string())?;
    orchestrator.play().map_err(|error| error.to_string())?;
    Ok(orchestrator.dashboard().clone())
}

#[tauri::command]
fn stop_fake_world(state: State<'_, AppState>) -> Result<PlayerDashboard, String> {
    let mut orchestrator = state
        .0
        .lock()
        .map_err(|_| "état applicatif indisponible".to_string())?;
    orchestrator.stop().map_err(|error| error.to_string())?;
    Ok(orchestrator.dashboard().clone())
}

#[tauri::command]
fn talk_to_fake_companion(
    state: State<'_, AppState>,
    companion_id: String,
    message: String,
) -> Result<String, String> {
    let orchestrator = state
        .0
        .lock()
        .map_err(|_| "état applicatif indisponible".to_string())?;
    orchestrator
        .conversation_reply(&companion_id, &message)
        .ok_or_else(|| "message vide ou compagnon inconnu".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = StateStore::open(data_dir.join("realmbox.sqlite"))?;
            let orchestrator = Orchestrator::new(store, FakeClientBackend::default())?;
            app.manage(AppState(Mutex::new(orchestrator)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            prepare_fake_world,
            start_fake_world,
            stop_fake_world,
            talk_to_fake_companion
        ])
        .run(tauri::generate_context!())
        .expect("RealmBox n'a pas pu initialiser sa boucle applicative");
}
