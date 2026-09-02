use std::path::Path;

use realmbox_client::{ClientBackend, ClientError};
use realmbox_domain::{
    Companion, CompanionRole, SetupSnapshot, SetupState, TransitionError, WorldPreset,
};
use realmbox_storage::{StateStore, StorageError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error(transparent)]
    Transition(#[from] TransitionError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Client(#[from] ClientError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDashboard {
    pub player_name: String,
    pub class_name: String,
    pub level: u8,
    pub preset: WorldPreset,
    pub local_ai_ready: bool,
    pub companions: Vec<Companion>,
    pub session_running: bool,
    pub evidence: String,
}

impl Default for PlayerDashboard {
    fn default() -> Self {
        Self {
            player_name: "Benjamin".into(),
            class_name: "Paladin".into(),
            level: 17,
            preset: WorldPreset::Living,
            local_ai_ready: true,
            companions: vec![
                Companion {
                    id: "thoran".into(),
                    name: "Thoran".into(),
                    role: CompanionRole::Tank,
                    level: 17,
                    ready: true,
                },
                Companion {
                    id: "melya".into(),
                    name: "Melya".into(),
                    role: CompanionRole::Healer,
                    level: 17,
                    ready: true,
                },
                Companion {
                    id: "kael".into(),
                    name: "Kael".into(),
                    role: CompanionRole::Damage,
                    level: 16,
                    ready: true,
                },
                Companion {
                    id: "lyra".into(),
                    name: "Lyra".into(),
                    role: CompanionRole::Damage,
                    level: 18,
                    ready: true,
                },
            ],
            session_running: false,
            evidence: "Runtime de démonstration — aucune donnée de jeu réelle".into(),
        }
    }
}

pub struct Orchestrator<B: ClientBackend> {
    store: StateStore,
    client: B,
    snapshot: SetupSnapshot,
    dashboard: PlayerDashboard,
}

impl<B: ClientBackend> Orchestrator<B> {
    pub fn new(store: StateStore, client: B) -> Result<Self, OrchestratorError> {
        let snapshot = store.load()?;
        Ok(Self {
            store,
            client,
            snapshot,
            dashboard: PlayerDashboard::default(),
        })
    }

    pub fn snapshot(&self) -> &SetupSnapshot {
        &self.snapshot
    }
    pub fn dashboard(&self) -> &PlayerDashboard {
        &self.dashboard
    }

    fn persist_transition(
        &mut self,
        state: SetupState,
        message: &str,
    ) -> Result<(), OrchestratorError> {
        self.snapshot.transition(state, message)?;
        self.store.save(&self.snapshot)?;
        Ok(())
    }

    pub fn run_fake_setup(&mut self) -> Result<Vec<SetupSnapshot>, OrchestratorError> {
        let mut timeline = Vec::new();
        if self.snapshot.state != SetupState::Uninitialized {
            return Ok(timeline);
        }
        for state in SetupState::setup_sequence().iter().skip(1) {
            self.persist_transition(*state, setup_message(*state))?;
            timeline.push(self.snapshot.clone());
        }
        self.client.prepare_runtime(
            Path::new("fake-game-data"),
            Path::new("fake-managed-runtime"),
        )?;
        self.client.configure_realm("127.0.0.1")?;
        self.client
            .install_addon(Path::new("addons/RealmBoxCompanions"))?;
        Ok(timeline)
    }

    pub fn play(&mut self) -> Result<Vec<SetupSnapshot>, OrchestratorError> {
        if self.client.kind() == realmbox_client::ClientKind::Fake {
            self.client.prepare_runtime(
                Path::new("fake-game-data"),
                Path::new("fake-managed-runtime"),
            )?;
            self.client.configure_realm("127.0.0.1")?;
        }
        let mut timeline = Vec::new();
        for state in SetupState::play_sequence().iter().skip(1) {
            self.persist_transition(*state, play_message(*state))?;
            timeline.push(self.snapshot.clone());
        }
        self.client.launch()?;
        self.dashboard.session_running = true;
        Ok(timeline)
    }

    pub fn stop(&mut self) -> Result<Vec<SetupSnapshot>, OrchestratorError> {
        self.client.stop()?;
        let mut timeline = Vec::new();
        for state in SetupState::stop_sequence().iter().skip(1) {
            self.persist_transition(*state, stop_message(*state))?;
            timeline.push(self.snapshot.clone());
        }
        self.dashboard.session_running = false;
        Ok(timeline)
    }

    pub fn recover(&mut self) -> Result<(), OrchestratorError> {
        if self.snapshot.state == SetupState::Ready {
            return Ok(());
        }
        self.persist_transition(SetupState::Error, "recovery.interrupted")?;
        self.persist_transition(SetupState::Recovering, "recovery.cleaning")?;
        self.client.stop()?;
        self.persist_transition(SetupState::Ready, "recovery.ready")?;
        Ok(())
    }

    pub fn conversation_reply(&self, companion_id: &str, text: &str) -> Option<String> {
        let companion = self
            .dashboard
            .companions
            .iter()
            .find(|companion| companion.id == companion_id)?;
        let normalized = text.trim();
        if normalized.is_empty() {
            return None;
        }
        let response = match companion.role {
            CompanionRole::Tank => format!(
                "Je passe devant, {name}. Restons groupés.",
                name = self.dashboard.player_name
            ),
            CompanionRole::Healer => {
                "Je suis prête. Gardons juste un peu de mana avant le prochain combat.".into()
            }
            CompanionRole::Damage => "Prêt. Je garde les yeux sur ta cible.".into(),
        };
        Some(format!("{} : {}", companion.name, response))
    }
}

fn setup_message(state: SetupState) -> &'static str {
    use SetupState::*;
    match state {
        InspectingEnvironment
        | DiscoveringGameData
        | SelectingGameData
        | ValidatingGameData
        | ImportingGameData => "setup.gameData",
        DownloadingClientRuntime
        | PreparingClientRuntime
        | DownloadingServerRuntime
        | PreparingServerRuntime
        | ExtractingServerData
        | ValidatingServerData => "setup.world",
        InitializingDatabase | MigratingDatabase | ConfiguringServer | ConfiguringPlayerbots => {
            "setup.inhabitants"
        }
        InstallingCompanionAddon => "setup.companions",
        DetectingHardware | RecommendingModel | PreparingOllama | DownloadingModel
        | BenchmarkingModel => "setup.localAi",
        CreatingLocalAccount | FinalizingSetup => "setup.finalCheck",
        Ready => "setup.ready",
        _ => "setup.progress",
    }
}

fn play_message(state: SetupState) -> &'static str {
    use SetupState::*;
    match state {
        StartingDatabase | StartingAuthServer | StartingWorldServer => "play.wakingWorld",
        StartingOllama => "play.localAi",
        ConfiguringClient | LaunchingClient | ConnectingClient => "play.openingGame",
        Running => "play.running",
        _ => "play.progress",
    }
}

fn stop_message(state: SetupState) -> &'static str {
    if state == SetupState::Ready {
        "stop.ready"
    } else {
        "stop.closing"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use realmbox_client::FakeClientBackend;

    #[test]
    fn complete_fake_player_journey_is_persisted() {
        let store = StateStore::in_memory().expect("store");
        let mut orchestrator =
            Orchestrator::new(store, FakeClientBackend::default()).expect("orchestrator");
        assert_eq!(
            orchestrator
                .run_fake_setup()
                .expect("setup")
                .last()
                .expect("ready")
                .state,
            SetupState::Ready
        );
        assert_eq!(
            orchestrator
                .play()
                .expect("play")
                .last()
                .expect("running")
                .state,
            SetupState::Running
        );
        assert!(orchestrator.dashboard().session_running);
        assert_eq!(
            orchestrator
                .stop()
                .expect("stop")
                .last()
                .expect("ready")
                .state,
            SetupState::Ready
        );
        assert!(!orchestrator.dashboard().session_running);
    }

    #[test]
    fn conversation_is_bounded_and_attributed() {
        let store = StateStore::in_memory().expect("store");
        let orchestrator =
            Orchestrator::new(store, FakeClientBackend::default()).expect("orchestrator");
        let reply = orchestrator
            .conversation_reply("melya", "On est prêts ?")
            .expect("reply");
        assert!(reply.starts_with("Melya :"));
        assert!(reply.len() < 180);
        assert_eq!(orchestrator.conversation_reply("melya", "  "), None);
    }

    #[test]
    fn launch_failure_is_reported() {
        let store = StateStore::in_memory().expect("store");
        let mut orchestrator =
            Orchestrator::new(store, FakeClientBackend::failing_launch()).expect("orchestrator");
        orchestrator.run_fake_setup().expect("setup");
        assert!(matches!(
            orchestrator.play(),
            Err(OrchestratorError::Client(_))
        ));
    }
}
