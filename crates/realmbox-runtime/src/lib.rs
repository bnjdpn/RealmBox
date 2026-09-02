use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("opération indisponible: {0}")]
    Unavailable(String),
    #[error("validation refusée: {0}")]
    Validation(String),
    #[error("délai dépassé pour {0}")]
    Timeout(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareSnapshot {
    pub operating_system: String,
    pub architecture: String,
    pub physical_cores: Option<u16>,
    pub logical_cores: u16,
    pub total_memory_bytes: Option<u64>,
    pub unified_memory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedProcess {
    pub pid: u32,
    pub executable: PathBuf,
    pub ownership_token: String,
}

pub trait PlatformPaths: Send + Sync {
    fn state_dir(&self) -> Result<PathBuf, RuntimeError>;
    fn logs_dir(&self) -> Result<PathBuf, RuntimeError>;
    fn cache_dir(&self) -> Result<PathBuf, RuntimeError>;
}

pub trait ProcessSupervisor: Send {
    fn start(&mut self, executable: &Path, args: &[String]) -> Result<OwnedProcess, RuntimeError>;
    fn wait_ready(&self, process: &OwnedProcess, timeout: Duration) -> Result<(), RuntimeError>;
    fn stop_owned(&mut self, process: &OwnedProcess, timeout: Duration)
    -> Result<(), RuntimeError>;
}

pub trait SecretStore: Send + Sync {
    fn put(&self, key: &str, secret: &[u8]) -> Result<(), RuntimeError>;
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, RuntimeError>;
    fn delete(&self, key: &str) -> Result<(), RuntimeError>;
}

pub trait HardwareInspector: Send + Sync {
    fn inspect(&self) -> Result<HardwareSnapshot, RuntimeError>;
}
pub trait ServerRuntimeManager: Send {
    fn start(&mut self) -> Result<(), RuntimeError>;
    fn stop(&mut self) -> Result<(), RuntimeError>;
    fn healthy(&self) -> Result<bool, RuntimeError>;
}
pub trait DatabaseRuntime: Send {
    fn start(&mut self) -> Result<(), RuntimeError>;
    fn migrate(&mut self) -> Result<(), RuntimeError>;
    fn stop(&mut self) -> Result<(), RuntimeError>;
    fn healthy(&self) -> Result<bool, RuntimeError>;
}
pub trait OllamaRuntime: Send {
    fn start(&mut self) -> Result<(), RuntimeError>;
    fn has_model(&self, model: &str) -> Result<bool, RuntimeError>;
    fn stop_if_owned(&mut self) -> Result<(), RuntimeError>;
    fn healthy(&self) -> Result<bool, RuntimeError>;
}
pub trait ManifestVerifier: Send + Sync {
    fn verify(&self, artifact: &Path, expected_sha256: &str) -> Result<(), RuntimeError>;
}
pub trait ArchiveExtractor: Send + Sync {
    fn extract_safe(&self, archive: &Path, destination: &Path) -> Result<(), RuntimeError>;
}
pub trait CodeSigningInspector: Send + Sync {
    fn verify(&self, binary: &Path) -> Result<(), RuntimeError>;
}
pub trait FilePermissionManager: Send + Sync {
    fn make_private(&self, path: &Path) -> Result<(), RuntimeError>;
}
pub trait NativeNotificationService: Send + Sync {
    fn notify(&self, title: &str, message: &str) -> Result<(), RuntimeError>;
}
pub trait PowerManagementGuard: Send {
    fn acquire(&mut self, reason: &str) -> Result<(), RuntimeError>;
    fn release(&mut self) -> Result<(), RuntimeError>;
}
pub trait GameDataValidator: Send + Sync {
    fn validate(&self, path: &Path) -> Result<(), RuntimeError>;
}
pub trait CompanionCommandGateway: Send {
    fn execute_allowlisted(&mut self, action: &str) -> Result<(), RuntimeError>;
}

#[derive(Debug, Default)]
pub struct FakeService {
    pub running: bool,
    pub healthy: bool,
    pub owned: bool,
    pub refuse_stop: bool,
}

impl FakeService {
    pub fn start(&mut self) -> Result<(), RuntimeError> {
        self.running = true;
        self.healthy = true;
        self.owned = true;
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), RuntimeError> {
        if self.refuse_stop {
            return Err(RuntimeError::Timeout("arrêt simulé".into()));
        }
        self.running = false;
        self.healthy = false;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MacOsHardwareInspector;

impl HardwareInspector for MacOsHardwareInspector {
    fn inspect(&self) -> Result<HardwareSnapshot, RuntimeError> {
        Ok(HardwareSnapshot {
            operating_system: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            physical_cores: None,
            logical_cores: std::thread::available_parallelism()
                .map(|count| count.get() as u16)
                .unwrap_or(1),
            total_memory_bytes: None,
            unified_memory: cfg!(all(target_os = "macos", target_arch = "aarch64")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_service_refuses_stop_without_claiming_success() {
        let mut service = FakeService {
            refuse_stop: true,
            ..FakeService::default()
        };
        service.start().expect("start");
        assert!(matches!(service.stop(), Err(RuntimeError::Timeout(_))));
        assert!(service.running);
    }

    #[test]
    fn apple_silicon_memory_is_marked_unified() {
        let snapshot = MacOsHardwareInspector.inspect().expect("inspect");
        assert_eq!(
            snapshot.unified_memory,
            cfg!(all(target_os = "macos", target_arch = "aarch64"))
        );
    }
}
