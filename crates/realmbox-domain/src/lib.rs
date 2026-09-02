use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupState {
    Uninitialized,
    InspectingEnvironment,
    DiscoveringGameData,
    SelectingGameData,
    ValidatingGameData,
    ImportingGameData,
    DownloadingClientRuntime,
    PreparingClientRuntime,
    DownloadingServerRuntime,
    PreparingServerRuntime,
    ExtractingServerData,
    ValidatingServerData,
    InitializingDatabase,
    MigratingDatabase,
    ConfiguringServer,
    ConfiguringPlayerbots,
    InstallingCompanionAddon,
    DetectingHardware,
    RecommendingModel,
    PreparingOllama,
    DownloadingModel,
    BenchmarkingModel,
    CreatingLocalAccount,
    FinalizingSetup,
    Ready,
    StartingDatabase,
    StartingOllama,
    StartingAuthServer,
    StartingWorldServer,
    ConfiguringClient,
    LaunchingClient,
    ConnectingClient,
    Running,
    StoppingClient,
    StoppingWorldServer,
    StoppingAuthServer,
    StoppingOllama,
    StoppingDatabase,
    BackingUp,
    Restoring,
    Repairing,
    UpdatingRuntime,
    Recovering,
    Error,
}

impl SetupState {
    pub fn setup_sequence() -> &'static [Self] {
        use SetupState::*;
        &[
            Uninitialized,
            InspectingEnvironment,
            DiscoveringGameData,
            SelectingGameData,
            ValidatingGameData,
            ImportingGameData,
            DownloadingClientRuntime,
            PreparingClientRuntime,
            DownloadingServerRuntime,
            PreparingServerRuntime,
            ExtractingServerData,
            ValidatingServerData,
            InitializingDatabase,
            MigratingDatabase,
            ConfiguringServer,
            ConfiguringPlayerbots,
            InstallingCompanionAddon,
            DetectingHardware,
            RecommendingModel,
            PreparingOllama,
            DownloadingModel,
            BenchmarkingModel,
            CreatingLocalAccount,
            FinalizingSetup,
            Ready,
        ]
    }

    pub fn play_sequence() -> &'static [Self] {
        use SetupState::*;
        &[
            Ready,
            StartingDatabase,
            StartingOllama,
            StartingAuthServer,
            StartingWorldServer,
            ConfiguringClient,
            LaunchingClient,
            ConnectingClient,
            Running,
        ]
    }

    pub fn stop_sequence() -> &'static [Self] {
        use SetupState::*;
        &[
            Running,
            StoppingClient,
            StoppingWorldServer,
            StoppingAuthServer,
            StoppingOllama,
            StoppingDatabase,
            Ready,
        ]
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        next_in(Self::setup_sequence(), self) == Some(next)
            || next_in(Self::play_sequence(), self) == Some(next)
            || next_in(Self::stop_sequence(), self) == Some(next)
            || matches!(next, Self::Error)
            || matches!((self, next), (Self::Error, Self::Recovering))
            || matches!((self, next), (Self::Recovering, Self::Ready))
            || matches!(
                (self, next),
                (Self::Ready, Self::BackingUp | Self::Repairing)
            )
            || matches!(
                (self, next),
                (Self::BackingUp | Self::Repairing, Self::Ready)
            )
    }

    pub fn progress(self) -> u8 {
        let sequence = Self::setup_sequence();
        sequence
            .iter()
            .position(|candidate| *candidate == self)
            .map(|position| ((position * 100) / (sequence.len() - 1)) as u8)
            .unwrap_or(if self == Self::Ready { 100 } else { 0 })
    }
}

fn next_in(sequence: &[SetupState], current: SetupState) -> Option<SetupState> {
    sequence
        .iter()
        .position(|candidate| *candidate == current)
        .and_then(|index| sequence.get(index + 1).copied())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransitionError {
    #[error("transition invalide de {from:?} vers {to:?}")]
    Invalid { from: SetupState, to: SetupState },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupSnapshot {
    pub state: SetupState,
    pub progress: u8,
    pub message_key: String,
    pub operation_id: u64,
    pub recoverable: bool,
}

impl Default for SetupSnapshot {
    fn default() -> Self {
        Self {
            state: SetupState::Uninitialized,
            progress: 0,
            message_key: "setup.welcome".into(),
            operation_id: 0,
            recoverable: false,
        }
    }
}

impl SetupSnapshot {
    pub fn transition(
        &mut self,
        next: SetupState,
        message_key: impl Into<String>,
    ) -> Result<(), TransitionError> {
        if !self.state.can_transition_to(next) {
            return Err(TransitionError::Invalid {
                from: self.state,
                to: next,
            });
        }
        self.state = next;
        self.progress = next.progress();
        self.message_key = message_key.into();
        self.operation_id = self.operation_id.saturating_add(1);
        self.recoverable = !matches!(
            next,
            SetupState::Ready | SetupState::Running | SetupState::Error
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorldPreset {
    Calm,
    Living,
    Crowded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompanionRole {
    Tank,
    Healer,
    Damage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Companion {
    pub id: String,
    pub name: String,
    pub role: CompanionRole,
    pub level: u8,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompanionIntent {
    FollowPlayer,
    HoldPosition,
    AttackPlayerTarget,
    Regroup,
    SetCombatStyle,
    SetManaPolicy,
    UseCooldowns,
    SaveCooldowns,
    ChangeRole,
    AcceptQuest,
    PrepareDungeon,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionCommand {
    Follow,
    Stay,
    Attack,
    Reset,
    CombatStyle(String),
    ManaPolicy(String),
    Cooldowns(bool),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IntentError {
    #[error("intention inconnue ou non exécutable")]
    Unknown,
    #[error("paramètre non autorisé")]
    InvalidParameter,
    #[error("action non disponible dans l'état actuel du groupe")]
    InvalidGroupState,
}

pub fn validate_intent(
    intent: CompanionIntent,
    parameter: Option<&str>,
    group_ready: bool,
) -> Result<CompanionCommand, IntentError> {
    if !group_ready && !matches!(intent, CompanionIntent::Regroup) {
        return Err(IntentError::InvalidGroupState);
    }
    match intent {
        CompanionIntent::FollowPlayer => Ok(CompanionCommand::Follow),
        CompanionIntent::HoldPosition => Ok(CompanionCommand::Stay),
        CompanionIntent::AttackPlayerTarget => Ok(CompanionCommand::Attack),
        CompanionIntent::Regroup => Ok(CompanionCommand::Reset),
        CompanionIntent::SetCombatStyle => match parameter {
            Some("prudent" | "offensif" | "equilibre") => Ok(CompanionCommand::CombatStyle(
                parameter.unwrap_or_default().into(),
            )),
            _ => Err(IntentError::InvalidParameter),
        },
        CompanionIntent::SetManaPolicy => match parameter {
            Some("conserver" | "normal") => Ok(CompanionCommand::ManaPolicy(
                parameter.unwrap_or_default().into(),
            )),
            _ => Err(IntentError::InvalidParameter),
        },
        CompanionIntent::UseCooldowns => Ok(CompanionCommand::Cooldowns(true)),
        CompanionIntent::SaveCooldowns => Ok(CompanionCommand::Cooldowns(false)),
        _ => Err(IntentError::Unknown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_sequence_reaches_ready_only_in_order() {
        let mut snapshot = SetupSnapshot::default();
        for state in SetupState::setup_sequence().iter().skip(1) {
            snapshot
                .transition(*state, "progress")
                .expect("valid transition");
        }
        assert_eq!(snapshot.state, SetupState::Ready);
        assert_eq!(snapshot.progress, 100);
    }

    #[test]
    fn rejects_invalid_transition() {
        let mut snapshot = SetupSnapshot::default();
        assert_eq!(
            snapshot.transition(SetupState::Running, "bad"),
            Err(TransitionError::Invalid {
                from: SetupState::Uninitialized,
                to: SetupState::Running
            })
        );
    }

    #[test]
    fn intent_mapping_is_allowlisted() {
        assert_eq!(
            validate_intent(CompanionIntent::FollowPlayer, None, true),
            Ok(CompanionCommand::Follow)
        );
        assert_eq!(
            validate_intent(CompanionIntent::SetCombatStyle, Some("shell"), true),
            Err(IntentError::InvalidParameter)
        );
        assert_eq!(
            validate_intent(CompanionIntent::Unknown, None, true),
            Err(IntentError::Unknown)
        );
    }
}
