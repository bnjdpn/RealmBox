use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientKind {
    OpenWow,
    OriginalWindows,
    Fake,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDataReport {
    pub compatible: bool,
    pub locale: Option<String>,
    pub detected_build: Option<u32>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientLaunch {
    pub process_id: u32,
    pub owned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDiagnostic {
    pub kind: ClientKind,
    pub available: bool,
    pub details: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("les données de jeu sont incompatibles")]
    IncompatibleData,
    #[error("le runtime client est indisponible: {0}")]
    RuntimeUnavailable(String),
    #[error("le client n'est pas pris en charge sur cette plateforme")]
    UnsupportedPlatform,
    #[error("échec du client: {0}")]
    Runtime(String),
}

pub trait ClientBackend: Send {
    fn kind(&self) -> ClientKind;
    fn validate_game_data(&self, path: &Path) -> Result<GameDataReport, ClientError>;
    fn prepare_runtime(&mut self, game_data: &Path, managed_dir: &Path) -> Result<(), ClientError>;
    fn configure_realm(&mut self, address: &str) -> Result<(), ClientError>;
    fn install_addon(&mut self, addon: &Path) -> Result<(), ClientError>;
    fn launch(&mut self) -> Result<ClientLaunch, ClientError>;
    fn is_ready(&self) -> Result<bool, ClientError>;
    fn has_exited(&self) -> Result<bool, ClientError>;
    fn stop(&mut self) -> Result<(), ClientError>;
    fn collect_logs(&self) -> Result<Vec<String>, ClientError>;
    fn repair(&mut self) -> Result<(), ClientError>;
    fn diagnose(&self) -> ClientDiagnostic;
}

#[derive(Debug, Default)]
pub struct OpenWowClientBackend {
    runtime: Option<PathBuf>,
    realm_address: Option<String>,
    launched: bool,
}

impl ClientBackend for OpenWowClientBackend {
    fn kind(&self) -> ClientKind {
        ClientKind::OpenWow
    }

    fn validate_game_data(&self, path: &Path) -> Result<GameDataReport, ClientError> {
        let data = path.join("Data");
        let mut issues = Vec::new();
        if !path.is_dir() {
            issues.push("Le dossier sélectionné n'existe pas".into());
        }
        if !data.is_dir() {
            issues.push("Le sous-dossier Data est manquant".into());
        }
        let locale = ["frFR", "enUS", "deDE", "esES", "ruRU"]
            .into_iter()
            .find(|locale| data.join(locale).is_dir())
            .map(str::to_owned);
        if data.is_dir() && locale.is_none() {
            issues.push("Aucune locale compatible reconnue".into());
        }
        Ok(GameDataReport {
            compatible: issues.is_empty(),
            locale,
            detected_build: None,
            issues,
        })
    }

    fn prepare_runtime(&mut self, game_data: &Path, managed_dir: &Path) -> Result<(), ClientError> {
        if !self.validate_game_data(game_data)?.compatible {
            return Err(ClientError::IncompatibleData);
        }
        self.runtime = Some(managed_dir.to_path_buf());
        Ok(())
    }

    fn configure_realm(&mut self, address: &str) -> Result<(), ClientError> {
        if !matches!(address, "127.0.0.1" | "localhost") {
            return Err(ClientError::Runtime(
                "seule une adresse locale est autorisée".into(),
            ));
        }
        self.realm_address = Some(address.into());
        Ok(())
    }

    fn install_addon(&mut self, addon: &Path) -> Result<(), ClientError> {
        if !addon.is_dir() {
            return Err(ClientError::RuntimeUnavailable("addon absent".into()));
        }
        Ok(())
    }

    fn launch(&mut self) -> Result<ClientLaunch, ClientError> {
        if self.runtime.is_none() || self.realm_address.is_none() {
            return Err(ClientError::RuntimeUnavailable(
                "runtime ou royaume non préparé".into(),
            ));
        }
        Err(ClientError::RuntimeUnavailable(
            "binaire OpenWoW non construit dans ce vertical slice".into(),
        ))
    }

    fn is_ready(&self) -> Result<bool, ClientError> {
        Ok(self.launched)
    }
    fn has_exited(&self) -> Result<bool, ClientError> {
        Ok(!self.launched)
    }
    fn stop(&mut self) -> Result<(), ClientError> {
        self.launched = false;
        Ok(())
    }
    fn collect_logs(&self) -> Result<Vec<String>, ClientError> {
        Ok(Vec::new())
    }
    fn repair(&mut self) -> Result<(), ClientError> {
        Ok(())
    }
    fn diagnose(&self) -> ClientDiagnostic {
        ClientDiagnostic {
            kind: self.kind(),
            available: self.runtime.is_some(),
            details: vec!["OpenWoW réel non lancé".into()],
        }
    }
}

#[derive(Debug, Default)]
pub struct OriginalWindowsClientBackend;

impl ClientBackend for OriginalWindowsClientBackend {
    fn kind(&self) -> ClientKind {
        ClientKind::OriginalWindows
    }
    fn validate_game_data(&self, _path: &Path) -> Result<GameDataReport, ClientError> {
        Err(ClientError::UnsupportedPlatform)
    }
    fn prepare_runtime(
        &mut self,
        _game_data: &Path,
        _managed_dir: &Path,
    ) -> Result<(), ClientError> {
        Err(ClientError::UnsupportedPlatform)
    }
    fn configure_realm(&mut self, _address: &str) -> Result<(), ClientError> {
        Err(ClientError::UnsupportedPlatform)
    }
    fn install_addon(&mut self, _addon: &Path) -> Result<(), ClientError> {
        Err(ClientError::UnsupportedPlatform)
    }
    fn launch(&mut self) -> Result<ClientLaunch, ClientError> {
        Err(ClientError::UnsupportedPlatform)
    }
    fn is_ready(&self) -> Result<bool, ClientError> {
        Ok(false)
    }
    fn has_exited(&self) -> Result<bool, ClientError> {
        Ok(true)
    }
    fn stop(&mut self) -> Result<(), ClientError> {
        Ok(())
    }
    fn collect_logs(&self) -> Result<Vec<String>, ClientError> {
        Ok(Vec::new())
    }
    fn repair(&mut self) -> Result<(), ClientError> {
        Err(ClientError::UnsupportedPlatform)
    }
    fn diagnose(&self) -> ClientDiagnostic {
        ClientDiagnostic {
            kind: self.kind(),
            available: false,
            details: vec!["Mode de compatibilité Windows uniquement".into()],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FakeClientBackend {
    events: VecDeque<String>,
    prepared: bool,
    launched: bool,
    fail_launch: bool,
}

impl FakeClientBackend {
    pub fn failing_launch() -> Self {
        Self {
            fail_launch: true,
            ..Self::default()
        }
    }
}

impl ClientBackend for FakeClientBackend {
    fn kind(&self) -> ClientKind {
        ClientKind::Fake
    }
    fn validate_game_data(&self, _path: &Path) -> Result<GameDataReport, ClientError> {
        Ok(GameDataReport {
            compatible: true,
            locale: Some("frFR".into()),
            detected_build: Some(12340),
            issues: vec![],
        })
    }
    fn prepare_runtime(
        &mut self,
        _game_data: &Path,
        _managed_dir: &Path,
    ) -> Result<(), ClientError> {
        self.prepared = true;
        self.events.push_back("runtime préparé".into());
        Ok(())
    }
    fn configure_realm(&mut self, address: &str) -> Result<(), ClientError> {
        if address != "127.0.0.1" {
            return Err(ClientError::Runtime(
                "le fake refuse les adresses non locales".into(),
            ));
        }
        self.events.push_back("royaume local configuré".into());
        Ok(())
    }
    fn install_addon(&mut self, _addon: &Path) -> Result<(), ClientError> {
        self.events.push_back("addon installé".into());
        Ok(())
    }
    fn launch(&mut self) -> Result<ClientLaunch, ClientError> {
        if self.fail_launch {
            return Err(ClientError::Runtime("crash simulé".into()));
        }
        if !self.prepared {
            return Err(ClientError::RuntimeUnavailable("fake non préparé".into()));
        }
        self.launched = true;
        self.events.push_back("client lancé".into());
        Ok(ClientLaunch {
            process_id: 4242,
            owned: true,
        })
    }
    fn is_ready(&self) -> Result<bool, ClientError> {
        Ok(self.launched)
    }
    fn has_exited(&self) -> Result<bool, ClientError> {
        Ok(!self.launched)
    }
    fn stop(&mut self) -> Result<(), ClientError> {
        self.launched = false;
        self.events.push_back("client arrêté".into());
        Ok(())
    }
    fn collect_logs(&self) -> Result<Vec<String>, ClientError> {
        Ok(self.events.iter().cloned().collect())
    }
    fn repair(&mut self) -> Result<(), ClientError> {
        self.fail_launch = false;
        self.events.push_back("client réparé".into());
        Ok(())
    }
    fn diagnose(&self) -> ClientDiagnostic {
        ClientDiagnostic {
            kind: self.kind(),
            available: true,
            details: self.events.iter().cloned().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openwow_validation_handles_unicode_and_does_not_claim_build() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Données privées");
        std::fs::create_dir_all(root.join("Data/frFR")).expect("fixture");
        let report = OpenWowClientBackend::default()
            .validate_game_data(&root)
            .expect("report");
        assert!(report.compatible);
        assert_eq!(report.locale.as_deref(), Some("frFR"));
        assert_eq!(report.detected_build, None);
    }

    #[test]
    fn fake_lifecycle_is_observable() {
        let mut client = FakeClientBackend::default();
        client
            .prepare_runtime(Path::new("source"), Path::new("managed"))
            .expect("prepare");
        client.configure_realm("127.0.0.1").expect("realm");
        client.launch().expect("launch");
        assert!(client.is_ready().expect("ready"));
        client.stop().expect("stop");
        assert!(client.has_exited().expect("exit"));
    }
}
