//! Read-only setup checks and a closed catalogue of help destinations.
use crate::launcher::CommandRunner;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationCheck {
    pub fresh_target: bool,
    pub platform_supported: bool,
    pub docker_ready: bool,
    pub compose_ready: bool,
    pub available_bytes: Option<u64>,
    pub required_bytes: u64,
    pub bot_capacity: Option<usize>,
}

pub fn inspect(
    runner: &impl CommandRunner,
    fresh_target: bool,
    platform_supported: bool,
    available_bytes: Option<u64>,
    required_bytes: u64,
) -> InstallationCheck {
    // No pull, container creation, database access, or Docker startup here.
    let docker = runner.run_bounded(
        "docker",
        &["info".into(), "--format".into(), "{{.MemTotal}}".into()],
        None,
        Duration::from_secs(10),
    );
    let docker_ready = docker.as_ref().is_ok_and(|value| !value.trim().is_empty());
    let bot_capacity = docker
        .as_ref()
        .ok()
        .filter(|value| value.trim().parse::<u64>().is_ok())
        .map(|value| crate::launcher::playerbot_capacity(value));
    let compose_ready = docker_ready
        && runner
            .run_bounded(
                "docker",
                &["compose".into(), "version".into(), "--short".into()],
                None,
                Duration::from_secs(10),
            )
            .is_ok_and(|value| !value.trim().is_empty());
    InstallationCheck {
        fresh_target,
        platform_supported,
        docker_ready,
        compose_ready,
        available_bytes,
        required_bytes,
        bot_capacity,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupResource {
    GameFr,
    GameEn,
    Docker,
}

impl SetupResource {
    pub fn url(&self) -> &'static str {
        match self {
            Self::GameFr => "https://chromiecraft.com/fr/telechargements/",
            Self::GameEn => "https://chromiecraft.com/en/downloads/",
            Self::Docker => "https://www.docker.com/products/docker-desktop/",
        }
    }
}

pub fn open_resource(runner: &impl CommandRunner, resource: SetupResource) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let (program, args) = ("/usr/bin/open", vec![resource.url().into()]);
    #[cfg(target_os = "windows")]
    let (program, args) = (
        "rundll32.exe",
        vec!["url.dll,FileProtocolHandler".into(), resource.url().into()],
    );
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let (program, args) = ("xdg-open", vec![resource.url().into()]);
    // The browser is an intentional OS handoff, not an owned realm process.
    // Do not put it in the kill-on-completion process group used by probes.
    runner.run(program, &args, None).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ffi::OsString, path::Path, sync::Mutex};
    struct Probe {
        replies: Mutex<Vec<Result<String, String>>>,
        calls: Mutex<Vec<String>>,
    }
    impl CommandRunner for Probe {
        fn run_long_with_env(
            &self,
            _: &Path,
            _: &[OsString],
            _: &[(OsString, OsString)],
            _: Option<&Path>,
            _: &Path,
        ) -> Result<(), String> {
            panic!("mutation")
        }
        fn spawn(
            &self,
            _: &Path,
            _: &[OsString],
            _: &[(OsString, OsString)],
            _: Option<&Path>,
            _: &Path,
        ) -> Result<u32, String> {
            panic!("mutation")
        }
        fn terminate(&self, _: u32) -> Result<(), String> {
            panic!("mutation")
        }
        fn is_process_running(&self, _: u32) -> Result<bool, String> {
            panic!("unexpected process inspection")
        }
        fn wait_service_tcp(&self, _: &Path, _: &str, _: u16, _: Duration) -> Result<(), String> {
            panic!("unexpected service access")
        }
        fn wait_tcp(&self, _: u16, _: Duration) -> Result<(), String> {
            panic!("unexpected network access")
        }
        fn run(
            &self,
            program: &str,
            args: &[OsString],
            _: Option<&Path>,
        ) -> Result<String, String> {
            assert_ne!(program, "docker", "probes must be time-bounded");
            self.calls
                .lock()
                .unwrap()
                .push(format!("{program} {args:?}"));
            self.replies.lock().unwrap().remove(0)
        }
        fn run_long(
            &self,
            _: &str,
            _: &[OsString],
            _: Option<&Path>,
            _: &Path,
        ) -> Result<(), String> {
            panic!("mutation")
        }
        fn run_bounded(
            &self,
            program: &str,
            args: &[OsString],
            _: Option<&Path>,
            timeout: Duration,
        ) -> Result<String, String> {
            assert_eq!(timeout, Duration::from_secs(10));
            self.calls
                .lock()
                .unwrap()
                .push(format!("{program} {args:?}"));
            self.replies.lock().unwrap().remove(0)
        }
    }
    fn probe(replies: Vec<Result<String, String>>) -> Probe {
        Probe {
            replies: Mutex::new(replies),
            calls: Mutex::new(vec![]),
        }
    }
    #[test]
    fn checks_only_read_only_docker_commands_and_reports_capacity() {
        let probe = probe(vec![Ok("17179869184".into()), Ok("2.39.0".into())]);
        let check = inspect(&probe, true, true, Some(100), 24);
        assert!(check.docker_ready && check.compose_ready && check.fresh_target);
        assert_eq!(check.bot_capacity, Some(50));
        let calls = probe.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert!(calls[0].contains("info") && calls[1].contains("version"));
    }
    #[test]
    fn unavailable_docker_or_disk_never_becomes_ready() {
        let probe = probe(vec![Err("timeout".into())]);
        let check = inspect(&probe, false, false, None, 24);
        assert!(
            !check.docker_ready
                && !check.compose_ready
                && !check.fresh_target
                && !check.platform_supported
        );
        assert_eq!(check.available_bytes, None);
        assert_eq!(check.bot_capacity, None);
        assert_eq!(probe.calls.lock().unwrap().len(), 1);
    }
    #[test]
    fn malformed_memory_does_not_invent_capacity_and_compose_can_fail() {
        let check = inspect(
            &probe(vec![Ok("unknown".into()), Err("missing".into())]),
            true,
            true,
            Some(2),
            24,
        );
        assert!(check.docker_ready);
        assert!(!check.compose_ready);
        assert_eq!(check.bot_capacity, None);
        assert!(check.available_bytes.unwrap() < check.required_bytes);
    }
    #[test]
    fn help_catalogue_rejects_arbitrary_urls_and_opens_only_a_fixed_destination() {
        assert!(serde_json::from_str::<SetupResource>("\"https://evil.example\"").is_err());
        let probe = probe(vec![Ok(String::new())]);
        open_resource(&probe, SetupResource::GameFr).unwrap();
        assert!(
            probe.calls.lock().unwrap()[0].contains("https://chromiecraft.com/fr/telechargements/")
        );
        assert!(SetupResource::GameEn.url().starts_with("https://"));
        assert!(SetupResource::Docker.url().starts_with("https://"));
    }
}
