use std::{
    ffi::OsString,
    fs::{self, File},
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};

use crate::ai::{self, AiCapability};

const INSTALL_SCHEMA: u32 = 3;
const RUNTIME_DIRECTORY: &str = "runtime-v3";
const SERVER_REPOSITORY: &str = "https://github.com/mod-playerbots/azerothcore-wotlk.git";
const SERVER_COMMIT: &str = "47960183bb03b83e8943eb2f0f39c16df9710c9d";
const PLAYERBOTS_REPOSITORY: &str = "https://github.com/mod-playerbots/mod-playerbots.git";
const PLAYERBOTS_COMMIT: &str = "2f7d9f774987d0157c6a0d0cc08c40bec3db3945";
const OLLAMA_CHAT_REPOSITORY: &str = "https://github.com/DustinHendrickson/mod-ollama-chat.git";
const OLLAMA_CHAT_COMMIT: &str = "a9d14b0b8955be136e657ac168dd255f5281a535";
const OLLAMA_PORT: u16 = 11435;
const MYSQL_IMAGE: &str =
    "mysql@sha256:b3b90af2a6552ae30c266fdb7d5dd55f3afb72404bb78d37fe8a23eb857fd3fb";
const AUTH_SERVER_IMAGE: Option<&str> = option_env!("REALMBOX_AUTH_SERVER_IMAGE");
const WORLD_SERVER_IMAGE: Option<&str> = option_env!("REALMBOX_WORLD_SERVER_IMAGE");
const DB_IMPORT_IMAGE: Option<&str> = option_env!("REALMBOX_DB_IMPORT_IMAGE");
const TOOLS_IMAGE: Option<&str> = option_env!("REALMBOX_TOOLS_IMAGE");
const DEFAULT_DOCKER_BUILD_JOBS: usize = 2;
const PLAYER_ACCOUNT_NAME: &str = "REALMBOX";
const PLAYER_ACCOUNT_PASSWORD: &str = "REALMBOX";
const SRP6_MODULUS: &str = "894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7";
const SUPPORTED_GAME_LOCALES: [&str; 10] = [
    "frFR", "enUS", "enGB", "deDE", "esES", "esMX", "ruRU", "koKR", "zhCN", "zhTW",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct ServerImages {
    authserver: String,
    worldserver: String,
    db_import: String,
    tools: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientChoice {
    #[default]
    ManagedOpenWow,
    OriginalWindows,
}

#[derive(Debug, Clone, Copy)]
struct PlatformAssets {
    label: &'static str,
    openwow_url: &'static str,
    openwow_archive: &'static str,
    openwow_sha256: &'static str,
    openwow_executable: &'static str,
    ollama_url: &'static str,
    ollama_archive: &'static str,
    ollama_sha256: &'static str,
    ollama_executable: &'static str,
}

fn platform_assets() -> Result<PlatformAssets, String> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok(PlatformAssets {
        label: "macOS Apple Silicon",
        openwow_url: "https://github.com/rkabachenko/OpenWow-snapshot/releases/download/v0.1.2/OpenWoW-0.1.2-macos-arm64.zip",
        openwow_archive: "OpenWoW-0.1.2-macos-arm64.zip",
        openwow_sha256: "832cb82fd853417ec64d8fd1a84cb8c6a91a57399fd4b87fb2e810a35b03ed18",
        openwow_executable: "OpenWoW.app/Contents/MacOS/openwow-client",
        ollama_url: "https://github.com/ollama/ollama/releases/download/v0.33.2/ollama-darwin.tgz",
        ollama_archive: "ollama-darwin-v0.33.2.tgz",
        ollama_sha256: "5751e296a2cd545939bdd51b700de0c20d319f0e723c9d7f48bebb5ab0b731d4",
        ollama_executable: "ollama",
    });

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok(PlatformAssets {
        label: "Windows x64",
        openwow_url: "https://github.com/rkabachenko/OpenWow-snapshot/releases/download/v0.1.2/OpenWoW-0.1.2-windows-x64.zip",
        openwow_archive: "OpenWoW-0.1.2-windows-x64.zip",
        openwow_sha256: "12e3b92eb49794cf69e7c39426030809387534b6257a49fcfb6d1ac953de2f0e",
        openwow_executable: "openwow-client.exe",
        ollama_url: "https://github.com/ollama/ollama/releases/download/v0.33.2/ollama-windows-amd64.zip",
        ollama_archive: "ollama-windows-amd64-v0.33.2.zip",
        ollama_sha256: "2439cbea65310b1aadf7d8fc41d7faf5d033f920d42e00a476c58bf9bff6950e",
        ollama_executable: "ollama.exe",
    });

    #[allow(unreachable_code)]
    Err("RealmBox prend actuellement en charge macOS Apple Silicon et Windows x64".into())
}

fn original_client_supported() -> bool {
    cfg!(all(target_os = "windows", target_arch = "x86_64"))
}

fn platform_label() -> String {
    platform_assets()
        .map(|assets| assets.label.to_owned())
        .unwrap_or_else(|_| format!("{} {}", std::env::consts::OS, std::env::consts::ARCH))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LauncherPhase {
    NeedsGameData,
    Installing,
    Ready,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ComponentState {
    Missing,
    Ready,
    Running,
    Stopped,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherComponent {
    pub id: &'static str,
    pub label: &'static str,
    pub state: ComponentState,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatus {
    pub phase: LauncherPhase,
    pub message: String,
    pub detail: Option<String>,
    pub progress: u8,
    pub installed: bool,
    pub bots_enabled: bool,
    pub bot_count: usize,
    pub ai_enabled: bool,
    pub ai_model: Option<String>,
    pub game_data_path: Option<String>,
    pub account_name: Option<&'static str>,
    pub account_password: Option<&'static str>,
    pub client_choice: ClientChoice,
    pub original_client_supported: bool,
    pub platform_label: String,
    pub components: Vec<LauncherComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherProgress {
    pub phase: LauncherPhase,
    pub message: String,
    pub detail: Option<String>,
    pub progress: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDataInspection {
    pub path: String,
    pub locale: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmDiagnostics {
    pub summary: String,
    pub component: &'static str,
    pub logs_path: String,
    pub recent_entries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationOptions {
    pub client_choice: ClientChoice,
    pub bots_enabled: bool,
    pub bot_count: usize,
    pub ai_enabled: bool,
    pub ai_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallationRecord {
    schema_version: u32,
    game_data_root: PathBuf,
    runtime_root: PathBuf,
    client_executable: PathBuf,
    client_choice: ClientChoice,
    compose_file: PathBuf,
    bots_enabled: bool,
    #[serde(default = "default_bot_count")]
    bot_count: usize,
    ai_enabled: bool,
    ai_model: Option<String>,
    ollama_executable: Option<PathBuf>,
    client_sha256: Option<String>,
    ollama_sha256: Option<String>,
    server_commit: String,
    playerbots_commit: String,
    ollama_chat_commit: Option<String>,
}

pub trait CommandRunner: Send + Sync + 'static {
    fn run(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
    ) -> Result<String, String>;
    fn run_long(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
        log_path: &Path,
    ) -> Result<(), String>;
    fn run_long_with_env(
        &self,
        program: &Path,
        args: &[OsString],
        environment: &[(OsString, OsString)],
        current_dir: Option<&Path>,
        log_path: &Path,
    ) -> Result<(), String>;
    fn spawn(
        &self,
        program: &Path,
        args: &[OsString],
        environment: &[(OsString, OsString)],
        current_dir: Option<&Path>,
        log_path: &Path,
    ) -> Result<u32, String>;
    fn terminate(&self, process_id: u32) -> Result<(), String>;
    fn is_process_running(&self, process_id: u32) -> Result<bool, String>;
    fn wait_tcp(&self, port: u16, timeout: Duration) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
    ) -> Result<String, String> {
        let mut command = Command::new(program);
        command.args(args);
        if let Some(directory) = current_dir {
            command.current_dir(directory);
        }
        let output = command
            .output()
            .map_err(|error| format!("impossible de lancer {program}: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "{program} a échoué ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn run_long(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
        log_path: &Path,
    ) -> Result<(), String> {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let log = File::create(log_path).map_err(|error| error.to_string())?;
        let errors = log.try_clone().map_err(|error| error.to_string())?;
        let mut command = Command::new(program);
        command
            .args(args)
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(errors));
        if let Some(directory) = current_dir {
            command.current_dir(directory);
        }
        let status = command
            .status()
            .map_err(|error| format!("impossible de lancer {program}: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "{program} a échoué ({status}); voir {}",
                log_path.display()
            ))
        }
    }

    fn run_long_with_env(
        &self,
        program: &Path,
        args: &[OsString],
        environment: &[(OsString, OsString)],
        current_dir: Option<&Path>,
        log_path: &Path,
    ) -> Result<(), String> {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let log = File::create(log_path).map_err(|error| error.to_string())?;
        let errors = log.try_clone().map_err(|error| error.to_string())?;
        let mut command = Command::new(program);
        command
            .args(args)
            .envs(environment.iter().cloned())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(errors));
        if let Some(directory) = current_dir {
            command.current_dir(directory);
        }
        let status = command
            .status()
            .map_err(|error| format!("impossible de lancer {}: {error}", program.display()))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "{} a échoué ({status}); voir {}",
                program.display(),
                log_path.display()
            ))
        }
    }

    fn spawn(
        &self,
        program: &Path,
        args: &[OsString],
        environment: &[(OsString, OsString)],
        current_dir: Option<&Path>,
        log_path: &Path,
    ) -> Result<u32, String> {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let log = File::create(log_path).map_err(|error| error.to_string())?;
        let errors = log.try_clone().map_err(|error| error.to_string())?;
        let mut child = Command::new(program)
            .args(args)
            .envs(environment.iter().cloned())
            .current_dir(current_dir.unwrap_or_else(|| Path::new(".")))
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(errors))
            .spawn()
            .map_err(|error| format!("impossible de lancer {}: {error}", program.display()))?;
        let process_id = child.id();
        thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(process_id)
    }

    fn terminate(&self, process_id: u32) -> Result<(), String> {
        #[cfg(unix)]
        return self
            .run(
                "kill",
                &[
                    OsString::from("-TERM"),
                    OsString::from(process_id.to_string()),
                ],
                None,
            )
            .map(|_| ());

        #[cfg(windows)]
        return self
            .run(
                "taskkill",
                &[
                    OsString::from("/PID"),
                    OsString::from(process_id.to_string()),
                    OsString::from("/T"),
                ],
                None,
            )
            .map(|_| ());

        #[allow(unreachable_code)]
        Err("arrêt de processus non pris en charge sur cette plateforme".into())
    }

    fn is_process_running(&self, process_id: u32) -> Result<bool, String> {
        #[cfg(unix)]
        {
            let status = Command::new("kill")
                .args(["-0", &process_id.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|error| {
                    format!("impossible d’inspecter le processus {process_id}: {error}")
                })?;
            Ok(status.success())
        }

        #[cfg(windows)]
        {
            let output = Command::new("tasklist")
                .args(["/FI", &format!("PID eq {process_id}"), "/FO", "CSV", "/NH"])
                .output()
                .map_err(|error| {
                    format!("impossible d’inspecter le processus {process_id}: {error}")
                })?;
            if !output.status.success() {
                return Err(format!(
                    "tasklist a échoué pendant l’inspection du processus {process_id}"
                ));
            }
            let expected = format!("\"{process_id}\"");
            Ok(String::from_utf8_lossy(&output.stdout)
                .split(',')
                .any(|field| field.trim() == expected))
        }
    }

    fn wait_tcp(&self, port: u16, timeout: Duration) -> Result<(), String> {
        let started = Instant::now();
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        while started.elapsed() < timeout {
            if TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_ok() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(500));
        }
        Err(format!(
            "le service local sur le port {port} n’est pas prêt après {} s",
            timeout.as_secs()
        ))
    }
}

pub struct LauncherService<R: CommandRunner> {
    app_data: PathBuf,
    addon_source: PathBuf,
    runner: R,
    client_process_id: Option<u32>,
    ai_process_id: Option<u32>,
}

impl<R: CommandRunner> LauncherService<R> {
    pub fn new(app_data: PathBuf, addon_source: PathBuf, runner: R) -> Result<Self, String> {
        fs::create_dir_all(&app_data).map_err(|error| error.to_string())?;
        Ok(Self {
            app_data,
            addon_source,
            runner,
            client_process_id: None,
            ai_process_id: None,
        })
    }

    pub fn inspect_ai_capability(&self) -> AiCapability {
        ai::inspect_ai_capability(&self.runner)
    }

    pub fn client_process_id(&self) -> Option<u32> {
        self.client_process_id
    }

    pub fn is_client_process_running(&self, process_id: u32) -> Result<bool, String> {
        if self.client_process_id != Some(process_id) {
            return Ok(false);
        }
        self.runner.is_process_running(process_id)
    }

    pub fn stop_after_client_exit<F>(
        &mut self,
        process_id: u32,
        progress: F,
    ) -> Result<Option<LauncherStatus>, String>
    where
        F: FnMut(LauncherProgress),
    {
        if self.client_process_id != Some(process_id) {
            return Ok(None);
        }
        self.client_process_id = None;
        self.stop(progress).map(Some)
    }

    pub fn status(&self) -> LauncherStatus {
        match self.load_record() {
            Ok(Some(record))
                if record.client_executable.is_file()
                    && record.compose_file.is_file()
                    && (!record.ai_enabled
                        || record
                            .ollama_executable
                            .as_ref()
                            .is_some_and(|path| path.is_file())) =>
            {
                self.installed_status(&record, LauncherPhase::Ready, "Installation prête", false)
            }
            Ok(Some(record)) => LauncherStatus {
                phase: LauncherPhase::Error,
                message: "Installation incomplète".into(),
                detail: Some("Un composant géré a disparu. Une réparation sera nécessaire.".into()),
                progress: 0,
                installed: false,
                bots_enabled: record.bots_enabled,
                bot_count: record.bot_count,
                ai_enabled: record.ai_enabled,
                ai_model: record.ai_model.clone(),
                game_data_path: Some(record.game_data_root.display().to_string()),
                account_name: None,
                account_password: None,
                client_choice: record.client_choice,
                original_client_supported: original_client_supported(),
                platform_label: platform_label(),
                components: components(
                    ComponentState::Error,
                    record.bots_enabled,
                    record.bot_count,
                    record.ai_enabled,
                    record.ai_model.as_deref(),
                    record.client_choice,
                ),
            },
            Ok(None) => missing_status(),
            Err(error) => LauncherStatus {
                phase: LauncherPhase::Error,
                message: "État local illisible".into(),
                detail: Some(error),
                progress: 0,
                installed: false,
                bots_enabled: true,
                bot_count: default_bot_count(),
                ai_enabled: false,
                ai_model: None,
                game_data_path: None,
                account_name: None,
                account_password: None,
                client_choice: ClientChoice::ManagedOpenWow,
                original_client_supported: original_client_supported(),
                platform_label: platform_label(),
                components: components(
                    ComponentState::Error,
                    true,
                    default_bot_count(),
                    false,
                    None,
                    ClientChoice::ManagedOpenWow,
                ),
            },
        }
    }

    pub fn bootstrap<F>(&mut self, progress: F) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        if self.load_record()?.is_some() {
            self.start(None, None, None, progress)
        } else {
            Ok(self.status())
        }
    }

    pub fn install<F>(
        &mut self,
        selected_path: &Path,
        options: InstallationOptions,
        mut progress: F,
    ) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        let InstallationOptions {
            client_choice,
            bots_enabled,
            bot_count,
            ai_enabled,
            ai_model,
        } = options;
        let platform = platform_assets()?;
        if client_choice == ClientChoice::OriginalWindows && !original_client_supported() {
            return Err("le client original fourni par le joueur est pris en charge uniquement sur Windows x64".into());
        }
        if ai_enabled && !bots_enabled {
            return Err("les dialogues IA nécessitent les compagnons Playerbots".into());
        }
        let ai_model = if ai_enabled {
            let model = ai_model
                .as_deref()
                .ok_or_else(|| "aucun modèle local n’a été recommandé".to_string())?;
            if !ai::is_allowed_ollama_model(model) {
                return Err("modèle Ollama refusé par la liste RealmBox".into());
            }
            Some(model.to_owned())
        } else {
            None
        };
        let game_data_root = validate_game_data_root(selected_path)?;
        self.emit(
            &mut progress,
            LauncherPhase::Installing,
            3,
            "Validation des données 3.3.5a",
            None,
        );
        self.runner
            .run(
                "docker",
                &[
                    "info".into(),
                    "--format".into(),
                    "{{.ServerVersion}}".into(),
                ],
                None,
            )
            .map_err(|error| format!("Docker Desktop doit être installé et démarré: {error}"))?;
        let docker_memory = self
            .runner
            .run(
                "docker",
                &["info".into(), "--format".into(), "{{.MemTotal}}".into()],
                None,
            )
            .unwrap_or_default();
        let bot_count = effective_playerbot_count(&docker_memory, bots_enabled, bot_count);
        let server_images = embedded_server_images()?;
        let docker_build_jobs = if server_images.is_some() {
            DEFAULT_DOCKER_BUILD_JOBS
        } else {
            docker_build_jobs(&docker_memory)
        };

        let staging = self.app_data.join(".installing-v3");
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
        }
        fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
        let logs = staging.join("logs");

        let result = (|| {
            self.emit(
                &mut progress,
                LauncherPhase::Installing,
                8,
                match client_choice {
                    ClientChoice::ManagedOpenWow => "Téléchargement du client OpenWoW",
                    ClientChoice::OriginalWindows => "Vérification du client fourni",
                },
                match client_choice {
                    ClientChoice::ManagedOpenWow => Some("Version 0.1.2 officielle"),
                    ClientChoice::OriginalWindows => Some("Aucun client propriétaire téléchargé"),
                },
            );
            let (staged_client_executable, client_sha256) = match client_choice {
                ClientChoice::ManagedOpenWow => {
                    let client_archive = staging.join(platform.openwow_archive);
                    self.runner.run_long(
                        "curl",
                        &[
                            "-L".into(),
                            "--fail".into(),
                            "--show-error".into(),
                            "--output".into(),
                            client_archive.as_os_str().into(),
                            platform.openwow_url.into(),
                        ],
                        None,
                        &logs.join("openwow-download.log"),
                    )?;
                    verify_sha256(&client_archive, platform.openwow_sha256)?;
                    let client_root = staging.join("client");
                    extract_zip(&self.runner, &client_archive, &client_root)?;
                    let executable = client_root.join(platform.openwow_executable);
                    if !executable.is_file() {
                        return Err(
                            "le ZIP OpenWoW vérifié ne contient pas l’exécutable attendu".into(),
                        );
                    }
                    verify_platform_client(&self.runner, &client_root)?;
                    fs::remove_file(&client_archive).map_err(|error| error.to_string())?;
                    (executable, Some(platform.openwow_sha256.to_owned()))
                }
                ClientChoice::OriginalWindows => {
                    let executable = game_data_root.join("Wow.exe");
                    if !executable.is_file() {
                        return Err("le dossier choisi ne contient pas Wow.exe".into());
                    }
                    (executable, None)
                }
            };

            let server_root = staging.join("server");
            if server_images.is_some() {
                self.emit(
                    &mut progress,
                    LauncherPhase::Installing,
                    22,
                    "Préparation du serveur précompilé",
                    Some("Images immuables adaptées à cette machine"),
                );
                fs::create_dir_all(server_root.join("env/dist/etc/modules"))
                    .map_err(|error| error.to_string())?;
                fs::create_dir_all(server_root.join("env/dist/logs"))
                    .map_err(|error| error.to_string())?;
            } else {
                self.emit(
                    &mut progress,
                    LauncherPhase::Installing,
                    22,
                    "Téléchargement du serveur épinglé",
                    Some("Mode développeur · AzerothCore Playerbots"),
                );
                clone_pinned(&self.runner, SERVER_REPOSITORY, SERVER_COMMIT, &server_root)?;
                self.emit(
                    &mut progress,
                    LauncherPhase::Installing,
                    31,
                    "Installation du module Playerbots",
                    Some("Commit immuable vérifié"),
                );
                let module_root = server_root.join("modules/mod-playerbots");
                clone_pinned(
                    &self.runner,
                    PLAYERBOTS_REPOSITORY,
                    PLAYERBOTS_COMMIT,
                    &module_root,
                )?;
                if ai_enabled {
                    self.emit(
                        &mut progress,
                        LauncherPhase::Installing,
                        35,
                        "Ajout des dialogues locaux",
                        Some("Module Ollama épinglé"),
                    );
                    clone_pinned(
                        &self.runner,
                        OLLAMA_CHAT_REPOSITORY,
                        OLLAMA_CHAT_COMMIT,
                        &server_root.join("modules/mod-ollama-chat"),
                    )?;
                }
                write_realmbox_dockerfile(&server_root)?;
            }

            let (uid, gid) = platform_container_ids(&self.runner)?;
            let database_password = secure_random_hex(24)?;
            write_secret_atomic(
                &server_root.join(".env"),
                format!("REALMBOX_DB_PASSWORD={database_password}\n").as_bytes(),
            )?;
            let compose = compose_file(
                uid.trim(),
                gid.trim(),
                &game_data_root,
                docker_build_jobs,
                server_images.as_ref(),
            );
            let compose_path = server_root.join("compose.realmbox.yaml");
            write_atomic(&compose_path, compose.as_bytes())?;

            let staged_ollama = if ai_enabled {
                self.emit(
                    &mut progress,
                    LauncherPhase::Installing,
                    38,
                    "Téléchargement du moteur de dialogue",
                    Some("Ollama 0.33.2 · exécution locale"),
                );
                let archive = staging.join(platform.ollama_archive);
                self.runner.run_long(
                    "curl",
                    &[
                        "-L".into(),
                        "--fail".into(),
                        "--show-error".into(),
                        "--output".into(),
                        archive.as_os_str().into(),
                        platform.ollama_url.into(),
                    ],
                    None,
                    &logs.join("ollama-download.log"),
                )?;
                verify_sha256(&archive, platform.ollama_sha256)?;
                let ai_root = staging.join("ai");
                fs::create_dir_all(&ai_root).map_err(|error| error.to_string())?;
                extract_ollama(&self.runner, &archive, &ai_root)?;
                let executable = ai_root.join(platform.ollama_executable);
                if !executable.is_file() {
                    return Err(
                        "l’archive Ollama vérifiée ne contient pas l’exécutable attendu".into(),
                    );
                }
                verify_platform_ollama(&self.runner, &ai_root)?;
                fs::remove_file(&archive).map_err(|error| error.to_string())?;
                Some(executable)
            } else {
                None
            };

            let managed_game = staging.join("game");
            prepare_game_for_client(
                &self.runner,
                &game_data_root,
                &managed_game,
                &self.addon_source,
                client_choice,
                &self.app_data.join("original-client-backup"),
            )?;

            self.emit(
                &mut progress,
                LauncherPhase::Installing,
                42,
                if server_images.is_some() {
                    "Téléchargement du serveur local"
                } else {
                    "Construction du serveur local"
                },
                if server_images.is_some() {
                    Some("Images précompilées vérifiées par digest")
                } else {
                    Some("Mode développeur · cette étape peut être longue")
                },
            );
            let server_action = if server_images.is_some() {
                [
                    "pull",
                    "db-import",
                    "authserver",
                    "worldserver",
                    "server-data-init",
                ]
            } else {
                [
                    "build",
                    "db-import",
                    "authserver",
                    "worldserver",
                    "server-data-init",
                ]
            };
            self.runner.run_long(
                "docker",
                &compose_args(&compose_path, &server_action),
                Some(&server_root),
                &logs.join(if server_images.is_some() {
                    "docker-pull.log"
                } else {
                    "docker-build.log"
                }),
            )?;

            self.emit(
                &mut progress,
                LauncherPhase::Installing,
                73,
                "Préparation de la base locale",
                Some("MySQL et données serveur vérifiées"),
            );
            self.runner.run_long(
                "docker",
                &compose_args(
                    &compose_path,
                    &["up", "-d", "--wait", "--wait-timeout", "180", "database"],
                ),
                Some(&server_root),
                &logs.join("database-start.log"),
            )?;
            self.runner.run_long(
                "docker",
                &compose_args(&compose_path, &["run", "--rm", "server-data-init"]),
                Some(&server_root),
                &logs.join("server-data.log"),
            )?;
            self.runner.run_long(
                "docker",
                &compose_args(&compose_path, &["run", "--rm", "db-import"]),
                Some(&server_root),
                &logs.join("database-import.log"),
            )?;
            write_playerbots_config(&server_root, bots_enabled, bot_count)?;
            write_ollama_chat_config(&server_root, ai_enabled, ai_model.as_deref())?;
            self.emit(
                &mut progress,
                LauncherPhase::Installing,
                88,
                "Création du compte local",
                Some("Identifiants réservés à cette machine"),
            );
            configure_local_account(
                &self.runner,
                &compose_path,
                &server_root,
                &logs.join("account-create.log"),
            )?;
            if let (Some(executable), Some(model)) = (staged_ollama.as_ref(), ai_model.as_deref()) {
                self.emit(
                    &mut progress,
                    LauncherPhase::Installing,
                    91,
                    "Préparation du modèle local",
                    Some(model),
                );
                pull_ollama_model(
                    &self.runner,
                    executable,
                    &staging.join("ai/models"),
                    model,
                    &logs,
                )?;
            }
            self.runner.run_long(
                "docker",
                &compose_args(&compose_path, &["down"]),
                Some(&server_root),
                &logs.join("database-stop.log"),
            )?;

            self.emit(
                &mut progress,
                LauncherPhase::Installing,
                94,
                "Finalisation de RealmBox",
                Some("Vérification des chemins gérés"),
            );
            let runtime_root = self.app_data.join(RUNTIME_DIRECTORY);
            if runtime_root.exists() {
                fs::remove_dir_all(&runtime_root).map_err(|error| error.to_string())?;
            }
            fs::rename(&staging, &runtime_root).map_err(|error| error.to_string())?;
            let record = InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: game_data_root.clone(),
                client_executable: match client_choice {
                    ClientChoice::ManagedOpenWow => runtime_root
                        .join("client")
                        .join(platform.openwow_executable),
                    ClientChoice::OriginalWindows => staged_client_executable,
                },
                client_choice,
                compose_file: runtime_root.join("server/compose.realmbox.yaml"),
                runtime_root: runtime_root.clone(),
                bots_enabled,
                bot_count,
                ai_enabled,
                ai_model: ai_model.clone(),
                ollama_executable: ai_enabled
                    .then(|| runtime_root.join("ai").join(platform.ollama_executable)),
                client_sha256,
                ollama_sha256: ai_enabled.then(|| platform.ollama_sha256.into()),
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: ai_enabled.then(|| OLLAMA_CHAT_COMMIT.into()),
            };
            self.save_record(&record)?;
            self.emit(
                &mut progress,
                LauncherPhase::Ready,
                100,
                "Installation terminée",
                Some("Au prochain lancement, RealmBox démarrera le monde automatiquement"),
            );
            Ok(self.installed_status(
                &record,
                LauncherPhase::Ready,
                "Installation terminée",
                false,
            ))
        })();

        if result.is_err() && staging.exists() {
            let compose_path = staging.join("server/compose.realmbox.yaml");
            if compose_path.is_file() {
                let _ = self.runner.run_long(
                    "docker",
                    &compose_args(&compose_path, &["down", "--volumes", "--remove-orphans"]),
                    compose_path.parent(),
                    &staging.join("install-rollback.log"),
                );
            }
        }
        result
    }

    pub fn start<F>(
        &mut self,
        bots_enabled: Option<bool>,
        bot_count: Option<usize>,
        ai_enabled: Option<bool>,
        mut progress: F,
    ) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        let mut record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        if let Some(enabled) = bots_enabled {
            record.bots_enabled = enabled;
        }
        if let Some(requested) = bot_count {
            record.bot_count = normalize_bot_count(requested);
        }
        if let Some(enabled) = ai_enabled {
            if enabled && !record.bots_enabled {
                return Err("les dialogues IA nécessitent les compagnons Playerbots".into());
            }
            if enabled && record.ollama_executable.is_none() {
                return Err("les dialogues IA n’ont pas été installés".into());
            }
            record.ai_enabled = enabled;
        }
        if !record.bots_enabled {
            record.ai_enabled = false;
        }
        self.save_record(&record)?;
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        ensure_worldserver_console(&record.compose_file)?;
        write_ollama_chat_config(server_root, record.ai_enabled, record.ai_model.as_deref())?;
        self.runner
            .run(
                "docker",
                &[
                    "info".into(),
                    "--format".into(),
                    "{{.ServerVersion}}".into(),
                ],
                None,
            )
            .map_err(|error| format!("Docker Desktop doit être démarré: {error}"))?;
        let docker_memory = self
            .runner
            .run(
                "docker",
                &["info".into(), "--format".into(), "{{.MemTotal}}".into()],
                None,
            )
            .unwrap_or_default();
        record.bot_count =
            effective_playerbot_count(&docker_memory, record.bots_enabled, record.bot_count);
        self.save_record(&record)?;
        write_playerbots_config(server_root, record.bots_enabled, record.bot_count)?;

        self.emit(
            &mut progress,
            LauncherPhase::Starting,
            12,
            "Démarrage de la base locale",
            None,
        );
        self.runner.run_long(
            "docker",
            &compose_args(
                &record.compose_file,
                &["up", "-d", "--wait", "--wait-timeout", "120", "database"],
            ),
            Some(server_root),
            &record.runtime_root.join("logs/start-database.log"),
        )?;

        self.emit(
            &mut progress,
            LauncherPhase::Starting,
            36,
            "Vérification du monde",
            None,
        );
        self.runner.run_long(
            "docker",
            &compose_args(&record.compose_file, &["run", "--rm", "server-data-init"]),
            Some(server_root),
            &record.runtime_root.join("logs/start-server-data.log"),
        )?;

        self.runner.run_long(
            "docker",
            &compose_args(&record.compose_file, &["run", "--rm", "db-import"]),
            Some(server_root),
            &record.runtime_root.join("logs/start-db-import.log"),
        )?;
        mark_local_realm_available(
            &self.runner,
            &record.compose_file,
            server_root,
            &record.runtime_root.join("logs/start-realm.log"),
        )?;

        if record.ai_enabled {
            self.emit(
                &mut progress,
                LauncherPhase::Starting,
                54,
                "Réveil des dialogues locaux",
                record.ai_model.as_deref(),
            );
            self.ai_process_id = Some(start_ollama(
                &self.runner,
                record
                    .ollama_executable
                    .as_deref()
                    .ok_or_else(|| "moteur Ollama absent".to_string())?,
                &record.runtime_root.join("ai/models"),
                &record.runtime_root.join("logs/ollama.log"),
                true,
            )?);
        }

        self.emit(
            &mut progress,
            LauncherPhase::Starting,
            61,
            if record.bots_enabled {
                "Réveil du serveur et des compagnons"
            } else {
                "Réveil du serveur"
            },
            None,
        );
        let server_start = (|| {
            self.runner.run_long(
                "docker",
                &compose_args(
                    &record.compose_file,
                    &["up", "-d", "authserver", "worldserver"],
                ),
                Some(server_root),
                &record.runtime_root.join("logs/start-server.log"),
            )?;
            self.runner.wait_tcp(3724, Duration::from_secs(180))?;
            self.runner.wait_tcp(8085, Duration::from_secs(180))
        })();
        if let Err(error) = server_start {
            if let Some(process_id) = self.ai_process_id.take() {
                let _ = self.runner.terminate(process_id);
            }
            return Err(error);
        }

        self.emit(
            &mut progress,
            LauncherPhase::Starting,
            88,
            "Ouverture du client",
            Some("Connexion locale 127.0.0.1"),
        );
        let managed_game = record.runtime_root.join("game");
        let (client_arguments, client_working_directory, client_log) = match record.client_choice {
            ClientChoice::ManagedOpenWow => (
                vec!["--game-data".into(), managed_game.as_os_str().into()],
                Some(managed_game.as_path()),
                "openwow.log",
            ),
            ClientChoice::OriginalWindows => (
                Vec::new(),
                Some(record.game_data_root.as_path()),
                "original-client.log",
            ),
        };
        let process_id = self
            .runner
            .spawn(
                &record.client_executable,
                &client_arguments,
                &[],
                client_working_directory,
                &record.runtime_root.join("logs").join(client_log),
            )
            .inspect_err(|_| {
                let _ = self.runner.run_long(
                    "docker",
                    &compose_args(
                        &record.compose_file,
                        &["stop", "worldserver", "authserver", "database"],
                    ),
                    Some(server_root),
                    &record.runtime_root.join("logs/start-rollback.log"),
                );
                if let Some(process_id) = self.ai_process_id.take() {
                    let _ = self.runner.terminate(process_id);
                }
            })?;
        self.client_process_id = Some(process_id);
        self.emit(
            &mut progress,
            LauncherPhase::Running,
            100,
            "Le monde est lancé",
            None,
        );
        Ok(self.installed_status(&record, LauncherPhase::Running, "Le monde est lancé", true))
    }

    pub fn stop<F>(&mut self, mut progress: F) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        let record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas installé".to_string())?;
        self.emit(
            &mut progress,
            LauncherPhase::Stopping,
            15,
            "Fermeture du client",
            None,
        );
        let client_stop = self
            .client_process_id
            .take()
            .map_or(Ok(()), |process_id| self.runner.terminate(process_id));
        self.emit(
            &mut progress,
            LauncherPhase::Stopping,
            50,
            "Arrêt du serveur local",
            None,
        );
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        let server_stop = self.runner.run_long(
            "docker",
            &compose_args(
                &record.compose_file,
                &["stop", "worldserver", "authserver", "database"],
            ),
            Some(server_root),
            &record.runtime_root.join("logs/stop.log"),
        );
        let ai_stop = self
            .ai_process_id
            .take()
            .map_or(Ok(()), |process_id| self.runner.terminate(process_id));
        client_stop.and(server_stop).and(ai_stop)?;
        self.emit(
            &mut progress,
            LauncherPhase::Ready,
            100,
            "Monde arrêté proprement",
            None,
        );
        Ok(self.installed_status(
            &record,
            LauncherPhase::Ready,
            "Monde arrêté proprement",
            false,
        ))
    }

    pub fn update_playerbot_population(
        &mut self,
        bots_enabled: bool,
        requested_count: usize,
    ) -> Result<LauncherStatus, String> {
        let mut record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas installé".to_string())?;
        let process_id = self.client_process_id.ok_or_else(|| {
            "le monde doit être lancé pour modifier la population à chaud".to_string()
        })?;
        if !self.runner.is_process_running(process_id)? {
            return Err("le client n’est plus en cours d’exécution".into());
        }
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        let container_id = self.runner.run(
            "docker",
            &compose_args(&record.compose_file, &["ps", "-q", "worldserver"]),
            Some(server_root),
        )?;
        let container_id = container_id.trim();
        if container_id.is_empty() {
            return Err("le serveur local n’est pas prêt".into());
        }
        let console_state = self.runner.run(
            "docker",
            &[
                "inspect".into(),
                "--format".into(),
                "{{.Config.OpenStdin}} {{.Config.Tty}}".into(),
                container_id.into(),
            ],
            None,
        )?;
        if console_state.trim() != "true false" {
            return Err(
                "redémarrez le monde une fois pour activer le contrôle des compagnons".into(),
            );
        }
        let docker_memory = self
            .runner
            .run(
                "docker",
                &["info".into(), "--format".into(), "{{.MemTotal}}".into()],
                None,
            )
            .unwrap_or_default();
        let effective_count =
            effective_playerbot_count(&docker_memory, bots_enabled, requested_count);
        write_playerbots_config(server_root, bots_enabled, effective_count)?;
        self.runner.run_long(
            "docker",
            &compose_args(
                &record.compose_file,
                &[
                    "exec",
                    "-T",
                    "worldserver",
                    "sh",
                    "-lc",
                    "printf 'playerbot rndbot reload\\nplayerbot rndbot update\\n' > /proc/1/fd/0",
                ],
            ),
            Some(server_root),
            &record.runtime_root.join("logs/playerbots-live-update.log"),
        )?;
        record.bots_enabled = bots_enabled;
        record.bot_count = effective_count;
        if !bots_enabled {
            record.ai_enabled = false;
        }
        self.save_record(&record)?;
        Ok(self.installed_status(
            &record,
            LauncherPhase::Running,
            "Population mise à jour",
            true,
        ))
    }

    pub fn diagnostics(&self) -> Result<RealmDiagnostics, String> {
        let record = self.load_record()?;
        let logs = record
            .as_ref()
            .map(|record| record.runtime_root.join("logs"))
            .unwrap_or_else(|| self.app_data.join("logs"));
        let recent_entries = filtered_log_entries(&logs, 40)?;
        let (component, summary) = diagnose_entries(&recent_entries, record.is_some());
        Ok(RealmDiagnostics {
            summary,
            component,
            logs_path: logs.display().to_string(),
            recent_entries,
        })
    }

    fn emit<F>(
        &self,
        progress: &mut F,
        phase: LauncherPhase,
        value: u8,
        message: &str,
        detail: Option<&str>,
    ) where
        F: FnMut(LauncherProgress),
    {
        progress(LauncherProgress {
            phase,
            message: message.into(),
            detail: detail.map(str::to_owned),
            progress: value,
        });
    }

    fn load_record(&self) -> Result<Option<InstallationRecord>, String> {
        let path = self.app_data.join("installation.json");
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path).map_err(|error| error.to_string())?;
        let schema_version = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|value| {
                value
                    .get("schemaVersion")
                    .and_then(serde_json::Value::as_u64)
            });
        if schema_version != Some(u64::from(INSTALL_SCHEMA)) {
            return Ok(None);
        }
        let record: InstallationRecord =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        Ok(Some(record))
    }

    fn save_record(&self, record: &InstallationRecord) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(record).map_err(|error| error.to_string())?;
        write_atomic(&self.app_data.join("installation.json"), &bytes)
    }

    fn installed_status(
        &self,
        record: &InstallationRecord,
        phase: LauncherPhase,
        message: &str,
        running: bool,
    ) -> LauncherStatus {
        LauncherStatus {
            phase,
            message: message.into(),
            detail: Some(
                if running {
                    "Serveur, base locale et client démarrés dans cet ordre."
                } else {
                    "Tous les composants requis sont installés et vérifiés."
                }
                .into(),
            ),
            progress: 100,
            installed: true,
            bots_enabled: record.bots_enabled,
            bot_count: record.bot_count,
            ai_enabled: record.ai_enabled,
            ai_model: record.ai_model.clone(),
            game_data_path: Some(record.game_data_root.display().to_string()),
            account_name: Some(PLAYER_ACCOUNT_NAME),
            account_password: Some(PLAYER_ACCOUNT_PASSWORD),
            client_choice: record.client_choice,
            original_client_supported: original_client_supported(),
            platform_label: platform_label(),
            components: components(
                if running {
                    ComponentState::Running
                } else {
                    ComponentState::Ready
                },
                record.bots_enabled,
                record.bot_count,
                record.ai_enabled,
                record.ai_model.as_deref(),
                record.client_choice,
            ),
        }
    }
}

fn missing_status() -> LauncherStatus {
    LauncherStatus {
        phase: LauncherPhase::NeedsGameData,
        message: "Données de jeu requises".into(),
        detail: Some("Choisissez le dossier de votre copie compatible avant l’installation des composants ouverts.".into()),
        progress: 0,
        installed: false,
        bots_enabled: true,
        bot_count: default_bot_count(),
        ai_enabled: false,
        ai_model: None,
        game_data_path: None,
        account_name: None,
        account_password: None,
        client_choice: ClientChoice::ManagedOpenWow,
        original_client_supported: original_client_supported(),
        platform_label: platform_label(),
        components: components(
            ComponentState::Missing,
            true,
            default_bot_count(),
            false,
            None,
            ClientChoice::ManagedOpenWow,
        ),
    }
}

fn components(
    state: ComponentState,
    bots_enabled: bool,
    bot_count: usize,
    ai_enabled: bool,
    ai_model: Option<&str>,
    client_choice: ClientChoice,
) -> Vec<LauncherComponent> {
    vec![
        LauncherComponent {
            id: "client",
            label: "Client de jeu",
            state,
            detail: match client_choice {
                ClientChoice::ManagedOpenWow => "OpenWoW géré et vérifié".into(),
                ClientChoice::OriginalWindows => "Client fourni par le joueur".into(),
            },
        },
        LauncherComponent {
            id: "database",
            label: "Sauvegarde du royaume",
            state,
            detail: "Conservée sur cette machine".into(),
        },
        LauncherComponent {
            id: "server",
            label: "Monde privé",
            state,
            detail: "Disponible hors ligne".into(),
        },
        LauncherComponent {
            id: "bots",
            label: "Compagnons",
            state: if bots_enabled {
                state
            } else {
                ComponentState::Stopped
            },
            detail: if bots_enabled {
                format!("{bot_count} aventuriers autonomes · équipe invocable en jeu")
            } else {
                "Désactivés par le joueur".into()
            },
        },
        LauncherComponent {
            id: "ai",
            label: "Dialogues vivants",
            state: if ai_enabled {
                state
            } else {
                ComponentState::Stopped
            },
            detail: if ai_enabled {
                format!(
                    "{} · calculé sur cette machine",
                    ai_model.unwrap_or("modèle local")
                )
            } else {
                "Désactivés par le joueur".into()
            },
        },
    ]
}

pub(crate) fn inspect_game_data_root(selected: &Path) -> Result<GameDataInspection, String> {
    let root = if selected
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("Data"))
    {
        selected.parent().unwrap_or(selected)
    } else {
        selected
    };
    let data = root.join("Data");
    if !data.is_dir() {
        return Err("le dossier choisi ne contient pas de sous-dossier Data".into());
    }

    for archive in ["common.MPQ", "expansion.MPQ", "lichking.MPQ"] {
        let path = find_file_case_insensitive(&data, archive)
            .ok_or_else(|| format!("archive WotLK requise absente : Data/{archive}"))?;
        validate_mpq_header(&path)?;
    }

    let (locale, locale_dir) = SUPPORTED_GAME_LOCALES
        .iter()
        .find_map(|locale| {
            let directory = find_directory_case_insensitive(&data, locale)?;
            find_file_case_insensitive(&directory, &format!("locale-{locale}.MPQ"))
                .map(|_| ((*locale).to_owned(), directory))
        })
        .ok_or_else(|| {
            "aucune archive de locale reconnue (par exemple Data/frFR/locale-frFR.MPQ)".to_string()
        })?;
    for archive in [
        format!("locale-{locale}.MPQ"),
        format!("lichking-locale-{locale}.MPQ"),
    ] {
        let path = find_file_case_insensitive(&locale_dir, &archive).ok_or_else(|| {
            format!("archive de locale WotLK requise absente : {locale}/{archive}")
        })?;
        validate_mpq_header(&path)?;
    }

    let canonical = fs::canonicalize(root).map_err(|error| error.to_string())?;
    Ok(GameDataInspection {
        path: canonical.display().to_string(),
        locale: locale.clone(),
        detail: format!(
            "Données WotLK {locale} reconnues ; la build 12340 sera confirmée par les extracteurs locaux."
        ),
    })
}

fn validate_game_data_root(selected: &Path) -> Result<PathBuf, String> {
    inspect_game_data_root(selected).map(|inspection| PathBuf::from(inspection.path))
}

fn find_file_case_insensitive(directory: &Path, expected: &str) -> Option<PathBuf> {
    fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(expected)
                && entry.path().is_file()
        })
        .map(|entry| entry.path())
}

fn find_directory_case_insensitive(directory: &Path, expected: &str) -> Option<PathBuf> {
    fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(expected)
                && entry.path().is_dir()
        })
        .map(|entry| entry.path())
}

fn validate_mpq_header(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|error| format!("{} : {error}", path.display()))?;
    let mut header = [0_u8; 4096];
    let read = file
        .read(&mut header)
        .map_err(|error| format!("{} : {error}", path.display()))?;
    let valid = (0..read.saturating_sub(3)).step_by(512).any(|offset| {
        let signature = &header[offset..offset + 4];
        signature == b"MPQ\x1a" || signature == b"MPQ\x1b"
    });
    if valid {
        Ok(())
    } else {
        Err(format!(
            "{} n’est pas une archive MPQ lisible",
            path.display()
        ))
    }
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    let actual = format!("{:x}", digest.finalize());
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "empreinte SHA-256 invalide pour {}: attendue {expected}, reçue {actual}",
            path.display()
        ))
    }
}

fn secure_random_hex(byte_count: usize) -> Result<String, String> {
    let mut bytes = vec![0_u8; byte_count];
    getrandom::fill(&mut bytes)
        .map_err(|error| format!("génération aléatoire sécurisée impossible: {error}"))?;
    Ok(encode_hex(&bytes))
}

fn platform_container_ids<R: CommandRunner>(runner: &R) -> Result<(String, String), String> {
    let _ = runner;
    // Docker Desktop runs a Linux VM. macOS group IDs such as 20 can already
    // belong to a system group in Ubuntu, so they must not be copied into the
    // image build. The upstream image is designed around its portable defaults.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    return Ok(("1000".into(), "1000".into()));

    #[cfg(all(unix, not(target_os = "macos")))]
    return Ok((
        runner.run("id", &["-u".into()], None)?,
        runner.run("id", &["-g".into()], None)?,
    ));

    #[allow(unreachable_code)]
    Err("plateforme de conteneur non prise en charge".into())
}

fn extract_zip<R: CommandRunner>(
    runner: &R,
    archive: &Path,
    destination: &Path,
) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    return runner
        .run(
            "ditto",
            &[
                "-x".into(),
                "-k".into(),
                archive.as_os_str().into(),
                destination.as_os_str().into(),
            ],
            None,
        )
        .map(|_| ());

    #[cfg(windows)]
    return runner
        .run(
            "powershell.exe",
            &[
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                "& { param($archive, $destination) Expand-Archive -LiteralPath $archive -DestinationPath $destination -Force }".into(),
                archive.as_os_str().into(),
                destination.as_os_str().into(),
            ],
            None,
        )
        .map(|_| ());

    #[allow(unreachable_code)]
    Err("extraction ZIP non prise en charge sur cette plateforme".into())
}

fn extract_ollama<R: CommandRunner>(
    runner: &R,
    archive: &Path,
    destination: &Path,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return runner
        .run(
            "tar",
            &[
                "-xzf".into(),
                archive.as_os_str().into(),
                "-C".into(),
                destination.as_os_str().into(),
            ],
            None,
        )
        .map(|_| ());

    #[cfg(windows)]
    return extract_zip(runner, archive, destination);

    #[allow(unreachable_code)]
    Err("extraction Ollama non prise en charge sur cette plateforme".into())
}

fn verify_platform_client<R: CommandRunner>(
    _runner: &R,
    _client_root: &Path,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return _runner
        .run(
            "codesign",
            &[
                "--verify".into(),
                "--deep".into(),
                "--strict".into(),
                _client_root.join("OpenWoW.app").as_os_str().into(),
            ],
            None,
        )
        .map(|_| ());

    #[cfg(windows)]
    return Ok(());

    #[allow(unreachable_code)]
    Err("vérification OpenWoW non prise en charge sur cette plateforme".into())
}

fn verify_platform_ollama<R: CommandRunner>(runner: &R, ai_root: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        for executable in [ai_root.join("ollama"), ai_root.join("llama-server")] {
            if !executable.is_file() {
                return Err(format!(
                    "l’archive Ollama ne contient pas {}",
                    executable.display()
                ));
            }
            runner.run(
                "codesign",
                &[
                    "--verify".into(),
                    "--strict".into(),
                    executable.as_os_str().into(),
                ],
                None,
            )?;
        }
        return Ok(());
    }

    #[cfg(windows)]
    return runner
        .run(
            "powershell.exe",
            &[
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                "& { param($binary) $signature = Get-AuthenticodeSignature -LiteralPath $binary; if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|, )O=Ollama Inc\\.(,|$)') { exit 1 } }".into(),
                ai_root.join("ollama.exe").as_os_str().into(),
            ],
            None,
        )
        .map(|_| ());

    #[allow(unreachable_code)]
    Err("vérification Ollama non prise en charge sur cette plateforme".into())
}

fn clone_pinned<R: CommandRunner>(
    runner: &R,
    repository: &str,
    commit: &str,
    destination: &Path,
) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    runner.run(
        "git",
        &["init".into(), destination.as_os_str().into()],
        None,
    )?;
    runner.run(
        "git",
        &[
            "-C".into(),
            destination.as_os_str().into(),
            "remote".into(),
            "add".into(),
            "origin".into(),
            repository.into(),
        ],
        None,
    )?;
    runner.run(
        "git",
        &[
            "-C".into(),
            destination.as_os_str().into(),
            "fetch".into(),
            "--depth".into(),
            "1".into(),
            "origin".into(),
            commit.into(),
        ],
        None,
    )?;
    runner.run(
        "git",
        &[
            "-C".into(),
            destination.as_os_str().into(),
            "checkout".into(),
            "--detach".into(),
            "FETCH_HEAD".into(),
        ],
        None,
    )?;
    let actual = runner.run(
        "git",
        &[
            "-C".into(),
            destination.as_os_str().into(),
            "rev-parse".into(),
            "HEAD".into(),
        ],
        None,
    )?;
    if actual.trim() != commit {
        return Err(format!(
            "commit inattendu pour {repository}: {}",
            actual.trim()
        ));
    }
    Ok(())
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("tmp");
    let mut file = File::create(&temporary).map_err(|error| error.to_string())?;
    file.write_all(contents)
        .map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;

    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }

    fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn write_secret_atomic(path: &Path, contents: &[u8]) -> Result<(), String> {
    write_atomic(path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn prepare_game_for_client<R: CommandRunner>(
    runner: &R,
    game_data_root: &Path,
    managed_game: &Path,
    addon_source: &Path,
    client_choice: ClientChoice,
    backup_root: &Path,
) -> Result<(), String> {
    match client_choice {
        ClientChoice::ManagedOpenWow => prepare_managed_openwow_game(
            runner,
            game_data_root,
            managed_game,
            addon_source,
            backup_root,
        ),
        ClientChoice::OriginalWindows => {
            prepare_original_client_files(game_data_root, addon_source, backup_root)
        }
    }
}

fn prepare_managed_openwow_game<R: CommandRunner>(
    runner: &R,
    game_data_root: &Path,
    managed_game: &Path,
    addon_source: &Path,
    backup_root: &Path,
) -> Result<(), String> {
    fs::create_dir_all(managed_game.join("WTF")).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    prepare_managed_openwow_data_overlay(game_data_root, managed_game)?;

    #[cfg(windows)]
    {
        runner.run(
            "cmd.exe",
            &[
                "/C".into(),
                "mklink".into(),
                "/J".into(),
                managed_game.join("Data").as_os_str().into(),
                game_data_root.join("Data").as_os_str().into(),
            ],
            None,
        )?;
        backup_and_write_local_realmlist(game_data_root, backup_root)?;
    }

    #[cfg(unix)]
    let _ = (runner, backup_root);

    write_atomic(
        &managed_game.join("WTF/Config.wtf"),
        b"SET realmlist \"127.0.0.1\"\nSET portal \"127.0.0.1\"\n",
    )?;
    install_companion_addon(addon_source, managed_game)
}

#[cfg(unix)]
fn prepare_managed_openwow_data_overlay(
    game_data_root: &Path,
    managed_game: &Path,
) -> Result<(), String> {
    let source_data = game_data_root.join("Data");
    let managed_data = managed_game.join("Data");
    fs::create_dir_all(&managed_data).map_err(|error| error.to_string())?;
    let mut localized_realmlist_count = 0;

    for entry in fs::read_dir(&source_data).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source = entry.path();
        let destination = managed_data.join(entry.file_name());
        let is_locale_directory = entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
            && source.join("realmlist.wtf").is_file();
        if !is_locale_directory {
            std::os::unix::fs::symlink(&source, &destination).map_err(|error| error.to_string())?;
            continue;
        }

        fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
        for locale_entry in fs::read_dir(&source).map_err(|error| error.to_string())? {
            let locale_entry = locale_entry.map_err(|error| error.to_string())?;
            if locale_entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("realmlist.wtf")
            {
                continue;
            }
            std::os::unix::fs::symlink(
                locale_entry.path(),
                destination.join(locale_entry.file_name()),
            )
            .map_err(|error| error.to_string())?;
        }
        write_atomic(
            &destination.join("realmlist.wtf"),
            b"set realmlist 127.0.0.1\nset portal 127.0.0.1\n",
        )?;
        localized_realmlist_count += 1;
    }

    if localized_realmlist_count == 0 {
        return Err("aucun realmlist localisable n’a été trouvé dans Data".into());
    }
    Ok(())
}

#[cfg(windows)]
fn backup_and_write_local_realmlist(
    game_data_root: &Path,
    backup_root: &Path,
) -> Result<(), String> {
    let locale = ["frFR", "enUS", "deDE", "esES", "ruRU"]
        .into_iter()
        .find(|locale| game_data_root.join("Data").join(locale).is_dir())
        .ok_or_else(|| "aucune locale compatible n’a été trouvée".to_string())?;
    let realmlist = game_data_root
        .join("Data")
        .join(locale)
        .join("realmlist.wtf");
    fs::create_dir_all(backup_root).map_err(|error| error.to_string())?;
    let backup = backup_root.join(format!("managed-openwow-realmlist-{locale}.wtf"));
    if realmlist.is_file() && !backup.exists() {
        fs::copy(&realmlist, backup).map_err(|error| error.to_string())?;
    }
    write_atomic(
        &realmlist,
        b"set realmlist 127.0.0.1\nset portal 127.0.0.1\n",
    )
}

fn prepare_original_client_files(
    game_data_root: &Path,
    addon_source: &Path,
    backup_root: &Path,
) -> Result<(), String> {
    if !game_data_root.join("Wow.exe").is_file() {
        return Err("le dossier choisi ne contient pas Wow.exe".into());
    }
    let locale = ["frFR", "enUS", "deDE", "esES", "ruRU"]
        .into_iter()
        .find(|locale| game_data_root.join("Data").join(locale).is_dir())
        .ok_or_else(|| "aucune locale compatible n’a été trouvée".to_string())?;
    let realmlist = game_data_root
        .join("Data")
        .join(locale)
        .join("realmlist.wtf");
    if realmlist.is_file() {
        fs::create_dir_all(backup_root).map_err(|error| error.to_string())?;
        let backup = backup_root.join(format!("realmlist-{locale}.wtf"));
        if !backup.exists() {
            fs::copy(&realmlist, backup).map_err(|error| error.to_string())?;
        }
    }
    write_atomic(&realmlist, b"set realmlist 127.0.0.1\n")?;
    install_companion_addon(addon_source, game_data_root)
}

fn install_companion_addon(addon_source: &Path, game_root: &Path) -> Result<(), String> {
    let addon_destination = game_root.join("Interface/AddOns/RealmBoxCompanions");
    fs::create_dir_all(&addon_destination).map_err(|error| error.to_string())?;
    for filename in [
        "RealmBoxCompanions.lua",
        "RealmBoxCompanions.toc",
        "RealmBoxCompanions.xml",
    ] {
        let source = addon_source.join(filename);
        if !source.is_file() {
            return Err(format!(
                "addon RealmBox absent du bundle: {}",
                source.display()
            ));
        }
        fs::copy(&source, addon_destination.join(filename)).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn write_playerbots_config(
    server_root: &Path,
    enabled: bool,
    requested_count: usize,
) -> Result<(), String> {
    let value = if enabled { 1 } else { 0 };
    let count = if enabled { requested_count.max(1) } else { 0 };
    write_module_config(
        server_root,
        "mod-playerbots",
        "playerbots.conf",
        &[
            ("AiPlayerbot.Enabled", value.to_string()),
            ("AiPlayerbot.RandomBotAutologin", value.to_string()),
            ("AiPlayerbot.MinRandomBots", count.to_string()),
            ("AiPlayerbot.MaxRandomBots", count.to_string()),
            ("AiPlayerbot.RandomBotGuildCount", "0".to_string()),
        ],
    )
}

fn ensure_worldserver_console(compose_path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(compose_path).map_err(|error| error.to_string())?;
    let mut updated = source.replace("    tty: true\n", "");
    let worldserver = updated
        .find("  worldserver:\n")
        .ok_or_else(|| "service worldserver absent du runtime géré".to_string())?;
    let environment = updated[worldserver..]
        .find("    environment:\n")
        .map(|offset| worldserver + offset)
        .ok_or_else(|| "configuration worldserver non reconnue".to_string())?;
    let service_header = &updated[worldserver..environment];
    if !service_header.contains("    stdin_open: true\n") {
        updated.insert_str(environment, "    stdin_open: true\n");
    }
    if updated == source {
        Ok(())
    } else {
        write_atomic(compose_path, updated.as_bytes())
    }
}

fn default_bot_count() -> usize {
    50
}

fn normalize_bot_count(requested: usize) -> usize {
    match requested {
        0..=5 => 5,
        6..=25 => 25,
        26..=50 => 50,
        51..=100 => 100,
        _ => 150,
    }
}

fn playerbot_capacity(memory_output: &str) -> usize {
    let memory_bytes = memory_output.trim().parse::<u64>().unwrap_or_default();
    const GIB: u64 = 1024 * 1024 * 1024;
    match memory_bytes {
        0 => 5,
        bytes if bytes < 12 * GIB => 5,
        bytes if bytes < 20 * GIB => 50,
        bytes if bytes < 28 * GIB => 100,
        _ => 150,
    }
}

fn effective_playerbot_count(memory_output: &str, enabled: bool, requested: usize) -> usize {
    if !enabled {
        return 0;
    }
    normalize_bot_count(requested).min(playerbot_capacity(memory_output))
}

fn filtered_log_entries(logs: &Path, limit: usize) -> Result<Vec<String>, String> {
    if !logs.is_dir() || limit == 0 {
        return Ok(Vec::new());
    }
    let mut files = fs::read_dir(logs)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "log")
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|entry| {
        std::cmp::Reverse(
            entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok(),
        )
    });
    let mut entries = Vec::new();
    for entry in files.into_iter().take(8) {
        let filename = entry.file_name().to_string_lossy().to_string();
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        for line in content
            .lines()
            .rev()
            .filter(|line| is_diagnostic_line(line))
            .take(10)
        {
            entries.push(format!("{filename} · {}", redact_diagnostic_line(line)));
            if entries.len() == limit {
                return Ok(entries);
            }
        }
    }
    Ok(entries)
}

fn is_diagnostic_line(line: &str) -> bool {
    let lowercase = line.to_ascii_lowercase();
    [
        "error", "failed", "failure", "fatal", "panic", "warning", "warn", "erreur", "échec",
    ]
    .iter()
    .any(|needle| lowercase.contains(needle))
}

fn redact_diagnostic_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.to_ascii_lowercase().contains("password")
        || trimmed.to_ascii_lowercase().contains("authorization")
        || trimmed.to_ascii_lowercase().contains("token=")
    {
        "[ligne sensible masquée]".into()
    } else {
        trimmed.chars().take(500).collect()
    }
}

fn diagnose_entries(entries: &[String], installed: bool) -> (&'static str, String) {
    if !installed {
        return (
            "launcher",
            "RealmBox attend une installation locale.".into(),
        );
    }
    let combined = entries.join("\n").to_ascii_lowercase();
    let component = if combined.contains("openwow") || combined.contains("client") {
        "client"
    } else if combined.contains("mysql") || combined.contains("database") {
        "database"
    } else if combined.contains("playerbot") || combined.contains("rndbot") {
        "bots"
    } else if combined.contains("ollama") || combined.contains("model") {
        "ai"
    } else if combined.contains("worldserver") || combined.contains("authserver") {
        "server"
    } else {
        "launcher"
    };
    let summary = if entries.is_empty() {
        "Aucune erreur récente détectée dans les journaux gérés.".into()
    } else {
        format!("{} événement(s) récent(s) à vérifier.", entries.len())
    };
    (component, summary)
}

fn write_ollama_chat_config(
    server_root: &Path,
    enabled: bool,
    model: Option<&str>,
) -> Result<(), String> {
    let source_exists = server_root
        .join("modules/mod-ollama-chat/conf/mod_ollama_chat.conf.dist")
        .is_file();
    let destination_exists = server_root
        .join("env/dist/etc/modules/mod_ollama_chat.conf")
        .is_file();
    if !enabled && !source_exists && !destination_exists {
        return Ok(());
    }
    if enabled && model.is_none() {
        return Err("modèle local absent de la configuration".into());
    }
    if let Some(model) = model
        && !ai::is_allowed_ollama_model(model)
    {
        return Err("modèle Ollama refusé par la liste RealmBox".into());
    }
    let enabled = u8::from(enabled).to_string();
    let model = model.unwrap_or("llama3.2:1b");
    write_module_config(
        server_root,
        "mod-ollama-chat",
        "mod_ollama_chat.conf",
        &[
            ("OllamaChat.Enable", enabled.clone()),
            (
                "OllamaChat.Url",
                format!("http://host.docker.internal:{OLLAMA_PORT}/api/generate"),
            ),
            ("OllamaChat.Model", model.to_owned()),
            ("OllamaChat.NumPredict", "72".into()),
            ("OllamaChat.ReasoningTokenReserve", "256".into()),
            ("OllamaChat.NumCtx", "2048".into()),
            ("OllamaChat.MaxConcurrentQueries", "1".into()),
            ("OllamaChat.DebugEnabled", "0".into()),
            ("OllamaChat.DebugShowFullPrompt", "0".into()),
            ("OllamaChat.BotReplyChance.Say", "0".into()),
            ("OllamaChat.BotReplyChance.Channel", "0".into()),
            ("OllamaChat.BotReplyChance.Party", "0".into()),
            ("OllamaChat.BotReplyChance.Guild", "0".into()),
            ("OllamaChat.EnableRandomChatter", enabled.clone()),
            ("OllamaChat.RandomChatterBotCommentChance", "2".into()),
            ("OllamaChat.RandomChatterMaxBotsPerPlayer", "1".into()),
            ("OllamaChat.EnableEventChatter", enabled),
            ("OllamaChat.EventChatterBotCommentChance", "10".into()),
            ("OllamaChat.EventChatterBotSelfCommentChance", "2".into()),
            ("OllamaChat.EventChatterMaxBotsPerPlayer", "1".into()),
            ("OllamaChat.EnableSentimentTracking", "0".into()),
        ],
    )
}

fn write_module_config(
    server_root: &Path,
    module_directory: &str,
    filename: &str,
    values: &[(&str, String)],
) -> Result<(), String> {
    let destination = server_root.join("env/dist/etc/modules").join(filename);
    let distributed = destination.with_file_name(format!("{filename}.dist"));
    let source = if destination.is_file() {
        destination.clone()
    } else if distributed.is_file() {
        distributed
    } else {
        server_root
            .join("modules")
            .join(module_directory)
            .join("conf")
            .join(format!("{filename}.dist"))
    };
    let mut config = fs::read_to_string(&source).map_err(|error| {
        format!(
            "configuration du module {module_directory} absente ({}): {error}",
            source.display()
        )
    })?;
    for (key, value) in values {
        replace_config_value(&mut config, key, value)?;
    }
    write_atomic(&destination, config.as_bytes())
}

fn replace_config_value(config: &mut String, key: &str, value: &str) -> Result<(), String> {
    let prefix = format!("{key} =");
    let had_trailing_newline = config.ends_with('\n');
    let mut replacements = 0;
    let mut lines = Vec::new();
    for line in config.lines() {
        if line.starts_with(&prefix) {
            lines.push(format!("{key} = {value}"));
            replacements += 1;
        } else {
            lines.push(line.to_owned());
        }
    }
    if replacements != 1 {
        return Err(format!(
            "la configuration épinglée doit contenir exactement une clé {key}"
        ));
    }
    *config = lines.join("\n");
    if had_trailing_newline {
        config.push('\n');
    }
    Ok(())
}

fn ollama_environment(models: &Path, local_only: bool) -> Vec<(OsString, OsString)> {
    let mut environment = vec![
        (
            "OLLAMA_HOST".into(),
            format!("127.0.0.1:{OLLAMA_PORT}").into(),
        ),
        ("OLLAMA_MODELS".into(), models.as_os_str().into()),
        ("OLLAMA_MAX_LOADED_MODELS".into(), "1".into()),
        ("OLLAMA_NUM_PARALLEL".into(), "1".into()),
        ("OLLAMA_MAX_QUEUE".into(), "8".into()),
    ];
    if local_only {
        environment.push(("OLLAMA_NO_CLOUD".into(), "true".into()));
    }
    environment
}

fn start_ollama<R: CommandRunner>(
    runner: &R,
    executable: &Path,
    models: &Path,
    log_path: &Path,
    local_only: bool,
) -> Result<u32, String> {
    let environment = ollama_environment(models, local_only);
    let process_id = runner.spawn(
        executable,
        &["serve".into()],
        &environment,
        executable.parent(),
        log_path,
    )?;
    if let Err(error) = runner.wait_tcp(OLLAMA_PORT, Duration::from_secs(45)) {
        let _ = runner.terminate(process_id);
        return Err(error);
    }
    Ok(process_id)
}

fn pull_ollama_model<R: CommandRunner>(
    runner: &R,
    executable: &Path,
    models: &Path,
    model: &str,
    logs: &Path,
) -> Result<(), String> {
    if !ai::is_allowed_ollama_model(model) {
        return Err("modèle Ollama refusé par la liste RealmBox".into());
    }
    fs::create_dir_all(models).map_err(|error| error.to_string())?;
    let process_id = start_ollama(
        runner,
        executable,
        models,
        &logs.join("ollama-serve.log"),
        false,
    )?;
    let environment = ollama_environment(models, false);
    let result = runner.run_long_with_env(
        executable,
        &["pull".into(), model.into()],
        &environment,
        executable.parent(),
        &logs.join("ollama-model-download.log"),
    );
    let stop_result = runner.terminate(process_id);
    result.and(stop_result)
}

fn compose_args(compose_file: &Path, trailing: &[&str]) -> Vec<OsString> {
    let mut args = vec![
        "compose".into(),
        "-p".into(),
        "realmbox-v3".into(),
        "-f".into(),
        compose_file.as_os_str().into(),
    ];
    args.extend(trailing.iter().map(OsString::from));
    args
}

fn configure_local_account<R: CommandRunner>(
    runner: &R,
    compose_file: &Path,
    server_root: &Path,
    log_path: &Path,
) -> Result<(), String> {
    let salt_hex = secure_random_hex(32)?;
    let salt = decode_hex_32(salt_hex.trim())?;
    let verifier = srp6_verifier(PLAYER_ACCOUNT_NAME, PLAYER_ACCOUNT_PASSWORD, &salt)?;
    let sql = format!(
        "INSERT IGNORE INTO account(username,salt,verifier,expansion,reg_mail,email,joindate) VALUES('{PLAYER_ACCOUNT_NAME}',UNHEX('{}'),UNHEX('{}'),2,'','',NOW()); UPDATE account SET salt=UNHEX('{}'),verifier=UNHEX('{}'),expansion=2 WHERE username='{PLAYER_ACCOUNT_NAME}'; INSERT IGNORE INTO realmcharacters(realmid,acctid,numchars) SELECT realmlist.id,account.id,0 FROM realmlist,account LEFT JOIN realmcharacters ON acctid=account.id WHERE account.username='{PLAYER_ACCOUNT_NAME}' AND acctid IS NULL; UPDATE realmlist SET name='RealmBox',address='127.0.0.1',localAddress='127.0.0.1',port=8085,flag=0,gamebuild=12340 WHERE id=1;",
        encode_hex(&salt),
        encode_hex(&verifier),
        encode_hex(&salt),
        encode_hex(&verifier),
    );
    let sql_environment = format!("REALMBOX_ACCOUNT_SQL={sql}");
    let mut args = compose_args(compose_file, &["exec", "-T", "-e"]);
    args.push(sql_environment.into());
    args.extend([
        "database".into(),
        "sh".into(),
        "-c".into(),
        r#"mysql --user=root --password="$MYSQL_ROOT_PASSWORD" --database=acore_auth --execute="$REALMBOX_ACCOUNT_SQL""#.into(),
    ]);
    runner.run_long("docker", &args, Some(server_root), log_path)
}

fn mark_local_realm_available<R: CommandRunner>(
    runner: &R,
    compose_file: &Path,
    server_root: &Path,
    log_path: &Path,
) -> Result<(), String> {
    let sql_environment = "REALMBOX_REALM_SQL=UPDATE realmlist SET name='RealmBox',address='127.0.0.1',localAddress='127.0.0.1',port=8085,flag=0,gamebuild=12340 WHERE id=1;";
    let mut args = compose_args(compose_file, &["exec", "-T", "-e"]);
    args.push(sql_environment.into());
    args.extend([
        "database".into(),
        "sh".into(),
        "-c".into(),
        r#"mysql --user=root --password="$MYSQL_ROOT_PASSWORD" --database=acore_auth --execute="$REALMBOX_REALM_SQL""#.into(),
    ]);
    runner.run_long("docker", &args, Some(server_root), log_path)
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("sel aléatoire invalide".into());
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "sel aléatoire invalide".to_string())?;
    }
    Ok(bytes)
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn srp6_verifier(username: &str, password: &str, salt: &[u8; 32]) -> Result<[u8; 32], String> {
    let credentials = format!(
        "{}:{}",
        username.to_ascii_uppercase(),
        password.to_ascii_uppercase()
    );
    let credentials_hash = Sha1::digest(credentials.as_bytes());
    let mut salted = Sha1::new();
    salted.update(salt);
    salted.update(credentials_hash);
    let exponent = BigUint::from_bytes_le(&salted.finalize());
    let modulus = BigUint::parse_bytes(SRP6_MODULUS.as_bytes(), 16)
        .ok_or_else(|| "paramètre SRP6 invalide".to_string())?;
    let verifier = BigUint::from(7_u8)
        .modpow(&exponent, &modulus)
        .to_bytes_le();
    let mut fixed = [0_u8; 32];
    if verifier.len() > fixed.len() {
        return Err("vérificateur SRP6 invalide".into());
    }
    fixed[..verifier.len()].copy_from_slice(&verifier);
    Ok(fixed)
}

fn docker_build_jobs(memory_output: &str) -> usize {
    let memory_bytes = memory_output.trim().parse::<u64>().unwrap_or_default();
    const GIB: u64 = 1024 * 1024 * 1024;
    match memory_bytes {
        0 => DEFAULT_DOCKER_BUILD_JOBS,
        bytes if bytes < 6 * GIB => 2,
        bytes if bytes < 10 * GIB => 3,
        bytes if bytes < 16 * GIB => 4,
        _ => 6,
    }
}

fn embedded_server_images() -> Result<Option<ServerImages>, String> {
    server_images_from_values([
        AUTH_SERVER_IMAGE,
        WORLD_SERVER_IMAGE,
        DB_IMPORT_IMAGE,
        TOOLS_IMAGE,
    ])
}

fn server_images_from_values(values: [Option<&str>; 4]) -> Result<Option<ServerImages>, String> {
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    if values.iter().any(Option::is_none) {
        return Err(
            "la release RealmBox doit embarquer les quatre images serveur ou aucune".into(),
        );
    }
    let values = values.map(|value| value.expect("les quatre images ont été vérifiées"));
    for value in values {
        validate_immutable_server_image(value)?;
    }
    Ok(Some(ServerImages {
        authserver: values[0].into(),
        worldserver: values[1].into(),
        db_import: values[2].into(),
        tools: values[3].into(),
    }))
}

fn validate_immutable_server_image(value: &str) -> Result<(), String> {
    let Some((repository, digest)) = value.rsplit_once("@sha256:") else {
        return Err(format!(
            "image serveur non immuable, digest SHA-256 requis: {value}"
        ));
    };
    if !repository.starts_with("ghcr.io/")
        || repository.len() <= "ghcr.io/".len()
        || !repository.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'/' | b'-' | b':')
        })
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("référence d’image serveur refusée: {value}"));
    }
    Ok(())
}

fn write_realmbox_dockerfile(server_root: &Path) -> Result<(), String> {
    let source_path = server_root.join("apps/docker/Dockerfile");
    let destination_path = server_root.join("apps/docker/Dockerfile.realmbox");
    let source = fs::read_to_string(&source_path).map_err(|error| {
        format!(
            "lecture du Dockerfile serveur impossible ({}): {error}",
            source_path.display()
        )
    })?;
    let arg_anchor = "ARG CTOOLS_BUILD=\"all\"";
    let build_anchor = "cmake --build . --config \"$CTYPE\" -j $(($(nproc) + 1))";
    let worldserver_anchor = "VOLUME /azerothcore/env/dist/etc\n\nCMD [\"worldserver\"]";
    let mmaps_anchor = "COPY --chown=$DOCKER_USER:$DOCKER_USER --from=build \\\n  /azerothcore/env/dist/bin/mmaps_generator /azerothcore/env/dist/bin/mmaps_generator";
    if !source.contains(arg_anchor)
        || !source.contains(build_anchor)
        || !source.contains(worldserver_anchor)
        || !source.contains(mmaps_anchor)
    {
        return Err("le Dockerfile serveur épinglé ne correspond pas au profil RealmBox".into());
    }
    let patched = source
        .replacen(
            arg_anchor,
            &format!("{arg_anchor}\nARG REALMBOX_BUILD_JOBS={DEFAULT_DOCKER_BUILD_JOBS}"),
            1,
        )
        .replacen(
            build_anchor,
            "cmake --build . --config \"$CTYPE\" --parallel \"$REALMBOX_BUILD_JOBS\"",
            1,
        )
        .replacen(
            worldserver_anchor,
            &format!(
                "COPY --chown=$DOCKER_USER:$DOCKER_USER \\\n     modules/mod-playerbots/data /azerothcore/modules/mod-playerbots/data\n\n{worldserver_anchor}"
            ),
            1,
        )
        .replacen(
            mmaps_anchor,
            &format!(
                "{mmaps_anchor}\n\nCOPY --chown=$DOCKER_USER:$DOCKER_USER \\\n  src/tools/mmaps_generator/mmaps-config.yaml /azerothcore/env/dist/bin/mmaps-config.yaml"
            ),
            1,
        );
    write_atomic(&destination_path, patched.as_bytes())
}

fn compose_file(
    uid: &str,
    gid: &str,
    game_data_root: &Path,
    docker_build_jobs: usize,
    images: Option<&ServerImages>,
) -> String {
    let game_data_mount = serde_json::to_string(&format!(
        "{}:/client-data:ro",
        game_data_root.join("Data").display()
    ))
    .expect("un chemin peut être sérialisé");
    let source = |target, image| compose_service_source(target, image, uid, gid, docker_build_jobs);
    COMPOSE_TEMPLATE
        .replace("__MYSQL_IMAGE__", MYSQL_IMAGE)
        .replace(
            "__TOOLS_SOURCE__",
            &source("tools", images.map(|images| images.tools.as_str())),
        )
        .replace(
            "__DB_IMPORT_SOURCE__",
            &source("db-import", images.map(|images| images.db_import.as_str())),
        )
        .replace(
            "__AUTH_SERVER_SOURCE__",
            &source(
                "authserver",
                images.map(|images| images.authserver.as_str()),
            ),
        )
        .replace(
            "__WORLD_SERVER_SOURCE__",
            &source(
                "worldserver",
                images.map(|images| images.worldserver.as_str()),
            ),
        )
        .replace("__SOURCE_ID__", &source_id(game_data_root))
        .replace("__GAME_DATA_MOUNT__", &game_data_mount)
}

fn compose_service_source(
    target: &str,
    image: Option<&str>,
    uid: &str,
    gid: &str,
    docker_build_jobs: usize,
) -> String {
    match image {
        Some(image) => format!("    image: {image}"),
        None => format!(
            "    build:\n      context: .\n      dockerfile: apps/docker/Dockerfile.realmbox\n      target: {target}\n      args: {{ USER_ID: {uid}, GROUP_ID: {gid}, DOCKER_USER: acore, REALMBOX_BUILD_JOBS: {docker_build_jobs} }}"
        ),
    }
}

fn source_id(game_data_root: &Path) -> String {
    let digest = Sha256::digest(game_data_root.as_os_str().as_encoded_bytes());
    format!("{:x}", digest)
}

const COMPOSE_TEMPLATE: &str = r#"services:
  database:
    image: __MYSQL_IMAGE__
    environment:
      MYSQL_ROOT_PASSWORD: ${REALMBOX_DB_PASSWORD}
    volumes:
      - realmbox-database:/var/lib/mysql
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "127.0.0.1", "-p${REALMBOX_DB_PASSWORD}"]
      interval: 3s
      timeout: 5s
      retries: 60

  server-data-init:
__TOOLS_SOURCE__
    # A named Docker volume starts owned by root. The extractors must initialise
    # it before the unprivileged runtime servers mount the generated data read-only.
    user: "0:0"
    working_dir: /work
    environment:
      REALMBOX_SOURCE_ID: __SOURCE_ID__
    volumes:
      - __GAME_DATA_MOUNT__
      - realmbox-server-data:/work
    command:
      - bash
      - -c
      - >-
        set -euo pipefail;
        if ! grep -Fxq "REALMBOX_SOURCE_ID=$${REALMBOX_SOURCE_ID}" /work/extraction-version 2>/dev/null; then
          ln -sfn /client-data /work/Data;
          /azerothcore/env/dist/bin/map_extractor;
          /azerothcore/env/dist/bin/vmap4_extractor;
          mkdir -p /work/vmaps;
          /azerothcore/env/dist/bin/vmap4_assembler /work/Buildings /work/vmaps;
          /azerothcore/env/dist/bin/mmaps_generator --config /azerothcore/env/dist/bin/mmaps-config.yaml --silent;
          rm -f /work/Data;
          echo "REALMBOX_SOURCE_ID=$${REALMBOX_SOURCE_ID}" > /work/extraction-version;
        fi

  db-import:
__DB_IMPORT_SOURCE__
    environment:
      AC_LOGIN_DATABASE_INFO: "database;3306;root;${REALMBOX_DB_PASSWORD};acore_auth"
      AC_WORLD_DATABASE_INFO: "database;3306;root;${REALMBOX_DB_PASSWORD};acore_world"
      AC_CHARACTER_DATABASE_INFO: "database;3306;root;${REALMBOX_DB_PASSWORD};acore_characters"
      AC_PLAYERBOTS_DATABASE_INFO: "database;3306;root;${REALMBOX_DB_PASSWORD};acore_playerbots"
    volumes:
      - ./env/dist/etc:/azerothcore/env/dist/etc
      - ./env/dist/logs:/azerothcore/env/dist/logs
    depends_on:
      database: { condition: service_healthy }

  authserver:
__AUTH_SERVER_SOURCE__
    environment:
      AC_LOGIN_DATABASE_INFO: "database;3306;root;${REALMBOX_DB_PASSWORD};acore_auth"
    ports:
      - "127.0.0.1:3724:3724"
    volumes:
      - ./env/dist/etc:/azerothcore/env/dist/etc
      - ./env/dist/logs:/azerothcore/env/dist/logs
    depends_on:
      database: { condition: service_healthy }

  worldserver:
__WORLD_SERVER_SOURCE__
    stdin_open: true
    environment:
      AC_LOGIN_DATABASE_INFO: "database;3306;root;${REALMBOX_DB_PASSWORD};acore_auth"
      AC_WORLD_DATABASE_INFO: "database;3306;root;${REALMBOX_DB_PASSWORD};acore_world"
      AC_CHARACTER_DATABASE_INFO: "database;3306;root;${REALMBOX_DB_PASSWORD};acore_characters"
      AC_PLAYERBOTS_DATABASE_INFO: "database;3306;root;${REALMBOX_DB_PASSWORD};acore_playerbots"
      AC_DATA_DIR: /azerothcore/env/dist/data
    ports:
      - "127.0.0.1:8085:8085"
    extra_hosts:
      - "host.docker.internal:host-gateway"
    volumes:
      - ./env/dist/etc:/azerothcore/env/dist/etc
      - ./env/dist/logs:/azerothcore/env/dist/logs
      - realmbox-server-data:/azerothcore/env/dist/data:ro
    depends_on:
      database: { condition: service_healthy }
      authserver: { condition: service_started }

volumes:
  realmbox-database:
  realmbox-server-data:
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn write_mpq(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("MPQ parent");
        }
        let mut bytes = b"MPQ\x1a".to_vec();
        bytes.resize(32, 0);
        fs::write(path, bytes).expect("MPQ fixture");
    }

    fn write_complete_game_data(root: &Path, locale: &str) {
        for archive in ["common.MPQ", "expansion.MPQ", "lichking.MPQ"] {
            write_mpq(&root.join("Data").join(archive));
        }
        for archive in [
            format!("locale-{locale}.MPQ"),
            format!("lichking-locale-{locale}.MPQ"),
        ] {
            write_mpq(&root.join("Data").join(locale).join(archive));
        }
    }

    #[derive(Default)]
    struct RecordingRunner {
        commands: Mutex<Vec<String>>,
    }

    impl CommandRunner for RecordingRunner {
        fn run(
            &self,
            program: &str,
            args: &[OsString],
            _current_dir: Option<&Path>,
        ) -> Result<String, String> {
            self.commands.lock().expect("commands").push(format!(
                "{program} {}",
                args.iter()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
            if program == "git" && args.iter().any(|arg| arg == "rev-parse") {
                return Ok(SERVER_COMMIT.into());
            }
            if program == "openssl" && args.iter().any(|arg| arg == "rand") {
                return Ok("00".repeat(32));
            }
            if program == "docker" && args.iter().any(|arg| arg == "ps") {
                return Ok("realmbox-worldserver-container".into());
            }
            if program == "docker" && args.iter().any(|arg| arg == "inspect") {
                return Ok("true false".into());
            }
            if program == "docker" && args.iter().any(|arg| arg == "{{.MemTotal}}") {
                return Ok("34359738368".into());
            }
            Ok(String::new())
        }
        fn run_long(
            &self,
            program: &str,
            args: &[OsString],
            _current_dir: Option<&Path>,
            _log_path: &Path,
        ) -> Result<(), String> {
            self.commands.lock().expect("commands").push(format!(
                "{program} {}",
                args.iter()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
            Ok(())
        }
        fn run_long_with_env(
            &self,
            program: &Path,
            args: &[OsString],
            environment: &[(OsString, OsString)],
            _current_dir: Option<&Path>,
            _log_path: &Path,
        ) -> Result<(), String> {
            self.commands.lock().expect("commands").push(format!(
                "{} {} [{}]",
                program.display(),
                args.iter()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" "),
                environment
                    .iter()
                    .map(|(key, value)| format!(
                        "{}={}",
                        key.to_string_lossy(),
                        value.to_string_lossy()
                    ))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
            Ok(())
        }
        fn spawn(
            &self,
            program: &Path,
            args: &[OsString],
            environment: &[(OsString, OsString)],
            current_dir: Option<&Path>,
            _log_path: &Path,
        ) -> Result<u32, String> {
            self.commands.lock().expect("commands").push(format!(
                "{} {} [{}] cwd={}",
                program.display(),
                args.iter()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" "),
                environment
                    .iter()
                    .map(|(key, value)| format!(
                        "{}={}",
                        key.to_string_lossy(),
                        value.to_string_lossy()
                    ))
                    .collect::<Vec<_>>()
                    .join(" "),
                current_dir
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<none>".into())
            ));
            Ok(42)
        }
        fn terminate(&self, process_id: u32) -> Result<(), String> {
            self.commands
                .lock()
                .expect("commands")
                .push(format!("terminate {process_id}"));
            Ok(())
        }
        fn is_process_running(&self, process_id: u32) -> Result<bool, String> {
            Ok(process_id == 42)
        }
        fn wait_tcp(&self, _port: u16, _timeout: Duration) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn validates_complete_wotlk_data_directory_or_its_parent() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let root = temporary.path().join("Jeu privé");
        write_complete_game_data(&root, "frFR");
        assert_eq!(
            validate_game_data_root(&root).expect("root"),
            fs::canonicalize(&root).expect("canonical")
        );
        assert_eq!(
            validate_game_data_root(&root.join("Data")).expect("data"),
            fs::canonicalize(&root).expect("canonical")
        );
        let inspection = inspect_game_data_root(&root).expect("inspection");
        assert_eq!(inspection.locale, "frFR");
        assert!(inspection.detail.contains("build 12340"));
    }

    #[test]
    fn rejects_incomplete_or_spoofed_game_data_before_installation() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let root = temporary.path().join("Wrath");
        write_complete_game_data(&root, "enGB");
        fs::write(root.join("Data/lichking.MPQ"), b"not an archive").expect("spoofed archive");
        assert!(
            inspect_game_data_root(&root)
                .expect_err("spoofed MPQ must fail")
                .contains("n’est pas une archive MPQ lisible")
        );

        write_mpq(&root.join("Data/lichking.MPQ"));
        fs::remove_file(root.join("Data/enGB/lichking-locale-enGB.MPQ"))
            .expect("remove locale archive");
        assert!(
            inspect_game_data_root(&root)
                .expect_err("incomplete locale must fail")
                .contains("lichking-locale-enGB.MPQ")
        );
    }

    #[test]
    fn original_client_configuration_is_backed_up_before_local_realm_change() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let game = temporary.path().join("Wrath");
        let locale = game.join("Data/frFR");
        let addon = temporary.path().join("addon");
        fs::create_dir_all(&locale).expect("locale");
        fs::create_dir_all(&addon).expect("addon");
        fs::write(game.join("Wow.exe"), "user binary").expect("client");
        fs::write(
            locale.join("realmlist.wtf"),
            "set realmlist example.invalid\n",
        )
        .expect("realmlist");
        for filename in [
            "RealmBoxCompanions.lua",
            "RealmBoxCompanions.toc",
            "RealmBoxCompanions.xml",
        ] {
            fs::write(addon.join(filename), filename).expect("addon fixture");
        }
        let backup = temporary.path().join("backup");

        prepare_original_client_files(&game, &addon, &backup).expect("prepare original client");

        assert_eq!(
            fs::read_to_string(backup.join("realmlist-frFR.wtf")).expect("backup"),
            "set realmlist example.invalid\n"
        );
        assert_eq!(
            fs::read_to_string(locale.join("realmlist.wtf")).expect("local realm"),
            "set realmlist 127.0.0.1\n"
        );
        assert!(
            game.join("Interface/AddOns/RealmBoxCompanions/RealmBoxCompanions.lua")
                .is_file()
        );
    }

    #[cfg(unix)]
    #[test]
    fn managed_openwow_uses_a_local_realmlist_without_changing_player_data() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let game = temporary.path().join("Wrath");
        let locale = game.join("Data/enUS");
        let managed_game = temporary.path().join("managed");
        let addon = temporary.path().join("addon");
        fs::create_dir_all(&locale).expect("locale");
        fs::create_dir_all(&addon).expect("addon");
        fs::write(game.join("Data/common.MPQ"), "player data").expect("data");
        fs::write(
            locale.join("realmlist.wtf"),
            "set realmlist logon.example.invalid\n",
        )
        .expect("source realmlist");
        fs::write(locale.join("locale-enUS.MPQ"), "locale data").expect("locale data");
        for filename in [
            "RealmBoxCompanions.lua",
            "RealmBoxCompanions.toc",
            "RealmBoxCompanions.xml",
        ] {
            fs::write(addon.join(filename), filename).expect("addon fixture");
        }

        prepare_managed_openwow_game(
            &RecordingRunner::default(),
            &game,
            &managed_game,
            &addon,
            &temporary.path().join("backup"),
        )
        .expect("managed OpenWoW game");

        assert_eq!(
            fs::read_to_string(locale.join("realmlist.wtf")).expect("source unchanged"),
            "set realmlist logon.example.invalid\n"
        );
        assert_eq!(
            fs::read_to_string(managed_game.join("Data/enUS/realmlist.wtf"))
                .expect("managed realmlist"),
            "set realmlist 127.0.0.1\nset portal 127.0.0.1\n"
        );
        assert!(
            fs::symlink_metadata(managed_game.join("Data/common.MPQ"))
                .expect("data link")
                .file_type()
                .is_symlink()
        );
        assert!(
            !fs::symlink_metadata(managed_game.join("Data/enUS"))
                .expect("locale overlay")
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn compose_pins_database_and_server_data_and_binds_ports_locally() {
        let game_root = Path::new("/Jeux privés/Wrath");
        let compose = compose_file("501", "20", game_root, 3, None);
        let expected_mount = serde_json::to_string(&format!(
            "{}:/client-data:ro",
            game_root.join("Data").display()
        ))
        .expect("mount fixture");
        assert!(compose.contains(MYSQL_IMAGE));
        assert!(compose.contains("Dockerfile.realmbox"));
        assert!(compose.contains("REALMBOX_BUILD_JOBS: 3"));
        assert!(compose.contains(&expected_mount));
        assert!(compose.contains("map_extractor"));
        assert!(compose.contains("mmaps_generator"));
        assert!(compose.contains(
            "mmaps_generator --config /azerothcore/env/dist/bin/mmaps-config.yaml --silent"
        ));
        assert!(compose.contains("server-data-init:\n"));
        assert!(compose.contains("    user: \"0:0\""));
        assert!(compose.contains("127.0.0.1:3724:3724"));
        assert!(compose.contains("127.0.0.1:8085:8085"));
        assert!(!compose.contains("3307:3306"));
        assert!(compose.contains("host.docker.internal:host-gateway"));
        assert!(compose.contains("    stdin_open: true\n"));
        assert!(!compose.contains("    tty: true\n"));
        assert!(!compose.contains("image: mysql:8.4"));
    }

    #[test]
    fn legacy_compose_is_migrated_to_an_attachable_worldserver_console() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let compose_path = temporary.path().join("compose.realmbox.yaml");
        let legacy = compose_file("501", "20", Path::new("/Games/Wrath"), 2, None)
            .replace("    stdin_open: true\n", "");
        fs::write(&compose_path, legacy).expect("legacy compose");

        ensure_worldserver_console(&compose_path).expect("migration");
        ensure_worldserver_console(&compose_path).expect("idempotent migration");

        let updated = fs::read_to_string(compose_path).expect("updated compose");
        assert_eq!(updated.matches("    stdin_open: true\n").count(), 1);
        assert_eq!(updated.matches("    tty: true\n").count(), 0);
    }

    #[test]
    fn release_images_are_complete_immutable_and_replace_every_server_build() {
        let digest = "a".repeat(64);
        let auth = format!("ghcr.io/realmbox/server-auth@sha256:{digest}");
        let world = format!("ghcr.io/realmbox/server-world@sha256:{digest}");
        let db_import = format!("ghcr.io/realmbox/server-db-import@sha256:{digest}");
        let tools = format!("ghcr.io/realmbox/server-tools@sha256:{digest}");
        let images =
            server_images_from_values([Some(&auth), Some(&world), Some(&db_import), Some(&tools)])
                .expect("valid release images")
                .expect("images");
        let compose = compose_file("1000", "1000", Path::new("/Games/Wrath"), 2, Some(&images));
        assert!(compose.contains(&format!("image: {auth}")));
        assert!(compose.contains(&format!("image: {world}")));
        assert!(compose.contains(&format!("image: {db_import}")));
        assert!(compose.contains(&format!("image: {tools}")));
        assert!(!compose.contains("build:"));

        assert!(server_images_from_values([Some(&auth), None, None, None]).is_err());
        assert!(
            server_images_from_values([
                Some("ghcr.io/realmbox/auth:latest"),
                Some(&world),
                Some(&db_import),
                Some(&tools),
            ])
            .is_err()
        );
        let injected = format!("ghcr.io/realmbox/auth\nimage@sha256:{digest}");
        assert!(
            server_images_from_values([
                Some(&injected),
                Some(&world),
                Some(&db_import),
                Some(&tools),
            ])
            .is_err()
        );
        assert_eq!(
            embedded_server_images()
                .expect("compile-time image set is valid")
                .is_some(),
            AUTH_SERVER_IMAGE.is_some()
        );
    }

    #[test]
    fn docker_build_parallelism_tracks_the_memory_given_to_docker() {
        assert_eq!(docker_build_jobs(""), DEFAULT_DOCKER_BUILD_JOBS);
        assert_eq!(docker_build_jobs("4294967296"), 2);
        assert_eq!(docker_build_jobs("8318976000"), 3);
        assert_eq!(docker_build_jobs("12884901888"), 4);
        assert_eq!(docker_build_jobs("34359738368"), 6);
    }

    #[test]
    fn playerbot_capacity_tracks_memory_available_to_docker() {
        assert_eq!(effective_playerbot_count("", true, 150), 5);
        assert_eq!(effective_playerbot_count("8589934592", true, 50), 5);
        assert_eq!(effective_playerbot_count("17179869184", true, 25), 25);
        assert_eq!(effective_playerbot_count("17179869184", true, 100), 50);
        assert_eq!(effective_playerbot_count("25769803776", true, 150), 100);
        assert_eq!(effective_playerbot_count("34359738368", true, 150), 150);
        assert_eq!(effective_playerbot_count("34359738368", false, 150), 0);
        assert_eq!(normalize_bot_count(12), 25);
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn docker_desktop_uses_the_portable_upstream_user() {
        let runner = RecordingRunner::default();
        assert_eq!(
            platform_container_ids(&runner).expect("container ids"),
            ("1000".into(), "1000".into())
        );
        assert!(runner.commands.lock().expect("commands").is_empty());
    }

    #[test]
    fn server_dockerfile_patch_is_bounded_and_deterministic() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let docker_dir = temporary.path().join("apps/docker");
        fs::create_dir_all(&docker_dir).expect("docker dir");
        fs::write(
            docker_dir.join("Dockerfile"),
            "ARG CTOOLS_BUILD=\"all\"\nRUN cmake --build . --config \"$CTYPE\" -j $(($(nproc) + 1))\nVOLUME /azerothcore/env/dist/etc\n\nCMD [\"worldserver\"]\nCOPY --chown=$DOCKER_USER:$DOCKER_USER --from=build \\\n  /azerothcore/env/dist/bin/mmaps_generator /azerothcore/env/dist/bin/mmaps_generator\n",
        )
        .expect("dockerfile");

        write_realmbox_dockerfile(temporary.path()).expect("patch");

        let patched =
            fs::read_to_string(docker_dir.join("Dockerfile.realmbox")).expect("patched dockerfile");
        assert!(patched.contains("ARG REALMBOX_BUILD_JOBS=2"));
        assert!(patched.contains("--parallel \"$REALMBOX_BUILD_JOBS\""));
        assert!(
            patched
                .contains("modules/mod-playerbots/data /azerothcore/modules/mod-playerbots/data")
        );
        assert!(patched.contains(
            "src/tools/mmaps_generator/mmaps-config.yaml /azerothcore/env/dist/bin/mmaps-config.yaml"
        ));
        assert!(!patched.contains("\n+     modules/mod-playerbots/data"));
        assert!(!patched.contains("$(nproc)"));
    }

    #[test]
    fn playerbots_configuration_uses_the_module_directory() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let source = temporary.path().join("modules/mod-playerbots/conf");
        fs::create_dir_all(&source).expect("source dir");
        fs::write(
            source.join("playerbots.conf.dist"),
            "AiPlayerbot.Enabled = 0\nAiPlayerbot.RandomBotAutologin = 0\nAiPlayerbot.MinRandomBots = 0\nAiPlayerbot.MaxRandomBots = 0\nAiPlayerbot.RandomBotGuildCount = 20\nAiPlayerbot.UnmanagedDefault = 42\n",
        )
        .expect("source config");
        write_playerbots_config(temporary.path(), true, 50).expect("playerbots config");
        let config = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/playerbots.conf"),
        )
        .expect("config");
        assert!(config.contains("AiPlayerbot.Enabled = 1"));
        assert!(config.contains("AiPlayerbot.MaxRandomBots = 50"));
        assert!(config.contains("AiPlayerbot.RandomBotGuildCount = 0"));
        assert!(config.contains("AiPlayerbot.UnmanagedDefault = 42"));
        assert!(
            !temporary
                .path()
                .join("env/dist/etc/playerbots.conf")
                .exists()
        );
    }

    #[test]
    fn prebuilt_module_configuration_uses_the_image_dist_file() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let modules = temporary.path().join("env/dist/etc/modules");
        fs::create_dir_all(&modules).expect("module dir");
        fs::write(
            modules.join("playerbots.conf.dist"),
            "AiPlayerbot.Enabled = 0\nAiPlayerbot.RandomBotAutologin = 0\nAiPlayerbot.MinRandomBots = 0\nAiPlayerbot.MaxRandomBots = 0\nAiPlayerbot.RandomBotGuildCount = 20\nAiPlayerbot.ImageDefault = keep\n",
        )
        .expect("image dist config");

        write_playerbots_config(temporary.path(), true, 5).expect("prebuilt config");

        let config = fs::read_to_string(modules.join("playerbots.conf")).expect("config");
        assert!(config.contains("AiPlayerbot.Enabled = 1"));
        assert!(config.contains("AiPlayerbot.ImageDefault = keep"));
    }

    #[test]
    fn ollama_chat_is_local_bounded_and_allowlisted() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let source = temporary.path().join("modules/mod-ollama-chat/conf");
        fs::create_dir_all(&source).expect("source dir");
        fs::write(
            source.join("mod_ollama_chat.conf.dist"),
            "OllamaChat.Enable = 0\nOllamaChat.Url = http://localhost\nOllamaChat.Model = test\nOllamaChat.NumPredict = 1\nOllamaChat.ReasoningTokenReserve = 1\nOllamaChat.NumCtx = 1\nOllamaChat.MaxConcurrentQueries = 0\nOllamaChat.DebugEnabled = 1\nOllamaChat.DebugShowFullPrompt = 1\nOllamaChat.BotReplyChance.Say = 1\nOllamaChat.BotReplyChance.Channel = 1\nOllamaChat.BotReplyChance.Party = 1\nOllamaChat.BotReplyChance.Guild = 1\nOllamaChat.EnableRandomChatter = 0\nOllamaChat.RandomChatterBotCommentChance = 1\nOllamaChat.RandomChatterMaxBotsPerPlayer = 2\nOllamaChat.EnableEventChatter = 0\nOllamaChat.EventChatterBotCommentChance = 1\nOllamaChat.EventChatterBotSelfCommentChance = 1\nOllamaChat.EventChatterMaxBotsPerPlayer = 2\nOllamaChat.EnableSentimentTracking = 1\nOllamaChat.UnmanagedDefault = keep\n",
        )
        .expect("source config");
        write_ollama_chat_config(temporary.path(), true, Some("qwen3:8b")).expect("valid model");
        let config = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/mod_ollama_chat.conf"),
        )
        .expect("config");
        assert!(config.contains("http://host.docker.internal:11435/api/generate"));
        assert!(config.contains("OllamaChat.MaxConcurrentQueries = 1"));
        assert!(config.contains("OllamaChat.BotReplyChance.Say = 0"));
        assert!(config.contains("OllamaChat.UnmanagedDefault = keep"));
        let environment = ollama_environment(Path::new("/managed/ai/models"), true);
        assert!(environment.contains(&(OsString::from("OLLAMA_NO_CLOUD"), OsString::from("true"))));
        assert!(environment.contains(&(OsString::from("OLLAMA_MAX_QUEUE"), OsString::from("8"))));
        assert!(
            write_ollama_chat_config(temporary.path(), true, Some("remote.example/model:latest"))
                .is_err()
        );
    }

    #[test]
    fn ollama_configuration_is_optional_when_the_module_was_not_installed() {
        let temporary = tempfile::tempdir().expect("tempdir");
        write_ollama_chat_config(temporary.path(), false, None).expect("disabled module");
        assert!(!temporary.path().join("env/dist/etc/modules").exists());
    }

    #[test]
    fn creates_the_same_srp6_verifier_as_the_pinned_server_algorithm() {
        let salt: [u8; 32] = std::array::from_fn(|index| index as u8);
        let verifier = srp6_verifier("realmbox", "realmbox", &salt).expect("verifier");
        assert_eq!(
            encode_hex(&verifier),
            "3610d7a68179ec15724ab50da984ec9ee8e5143d36aa594672b9d3ec45b0ab21"
        );
    }

    #[test]
    fn account_creation_uses_the_local_database_and_is_idempotent() {
        let runner = RecordingRunner::default();
        configure_local_account(
            &runner,
            Path::new("/managed/server/compose.realmbox.yaml"),
            Path::new("/managed/server"),
            Path::new("/managed/logs/account.log"),
        )
        .expect("account command");
        let commands = runner.commands.lock().expect("commands");
        assert!(commands[0].contains("docker compose -p realmbox"));
        assert!(commands[0].contains("INSERT IGNORE INTO account"));
        assert!(commands[0].contains("UPDATE account SET salt=UNHEX"));
        assert!(commands[0].contains("UPDATE realmlist SET name='RealmBox'"));
        assert!(commands[0].contains("flag=0"));
    }

    #[test]
    fn local_realm_is_made_available_before_each_start() {
        let runner = RecordingRunner::default();
        mark_local_realm_available(
            &runner,
            Path::new("/managed/server/compose.realmbox.yaml"),
            Path::new("/managed/server"),
            Path::new("/managed/logs/realm.log"),
        )
        .expect("realm command");
        let commands = runner.commands.lock().expect("commands");
        assert!(commands[0].contains("UPDATE realmlist SET name='RealmBox'"));
        assert!(commands[0].contains("flag=0"));
    }

    #[cfg(unix)]
    #[test]
    fn database_secret_is_not_group_or_world_readable() {
        use std::os::unix::fs::PermissionsExt;

        let temporary = tempfile::tempdir().expect("tempdir");
        let secret = temporary.path().join(".env");
        write_secret_atomic(&secret, b"REALMBOX_DB_PASSWORD=test\n").expect("secret");
        let mode = fs::metadata(secret).expect("metadata").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn compose_command_order_keeps_runtime_details_typed() {
        let path = Path::new("/managed/server/compose.realmbox.yaml");
        let args = compose_args(
            path,
            &["up", "-d", "--wait", "--wait-timeout", "120", "database"],
        );
        assert_eq!(args[0], "compose");
        assert_eq!(args[1], "-p");
        assert_eq!(args[2], "realmbox-v3");
        assert_eq!(args.last(), Some(&OsString::from("database")));
        assert!(args.contains(&OsString::from("--wait")));
        let runner = RecordingRunner::default();
        runner.run("docker", &args, None).expect("recorded");
        assert!(
            runner.commands.lock().expect("commands")[0]
                .starts_with("docker compose -p realmbox-v3")
        );
    }

    #[test]
    fn client_exit_stops_the_owned_world_and_ai_process() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join("runtime-v2");
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::write(
            &compose_file,
            "services:\n  worldserver:\n    environment:\n      TEST: value\n",
        )
        .expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");
        service
            .save_record(&InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: temporary.path().join("game-source"),
                runtime_root,
                client_executable,
                client_choice: ClientChoice::ManagedOpenWow,
                compose_file,
                bots_enabled: true,
                bot_count: 50,
                ai_enabled: true,
                ai_model: Some("qwen3:8b".into()),
                ollama_executable: Some(temporary.path().join("ai/ollama")),
                client_sha256: Some("test-openwow-sha256".into()),
                ollama_sha256: Some("test-ollama-sha256".into()),
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: Some(OLLAMA_CHAT_COMMIT.into()),
            })
            .expect("record");
        service.client_process_id = Some(42);
        service.ai_process_id = Some(84);

        let status = service
            .stop_after_client_exit(42, |_| {})
            .expect("stop")
            .expect("owned client");
        assert_eq!(status.phase, LauncherPhase::Ready);
        let commands = service.runner.commands.lock().expect("commands");
        assert!(
            commands
                .iter()
                .any(|command| command.contains("docker compose"))
        );
        assert!(commands.iter().any(|command| command == "terminate 84"));
        assert!(!commands.iter().any(|command| command == "terminate 42"));
    }

    #[cfg(unix)]
    #[test]
    fn completed_client_process_is_reaped_and_reported_stopped() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = SystemCommandRunner;
        let process_id = runner
            .spawn(
                Path::new("/bin/sh"),
                &["-c".into(), "exit 0".into()],
                &[],
                None,
                &temporary.path().join("client.log"),
            )
            .expect("spawn client");

        for _ in 0..40 {
            if !runner
                .is_process_running(process_id)
                .expect("inspect client")
            {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("completed client process remained visible after one second");
    }

    #[test]
    fn managed_openwow_starts_from_its_writable_game_root() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        let managed_game = runtime_root.join("game");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::create_dir_all(&managed_game).expect("managed game");
        let playerbots_config = runtime_root.join("server/env/dist/etc/modules/playerbots.conf");
        fs::create_dir_all(playerbots_config.parent().expect("module config"))
            .expect("module config");
        fs::write(
            &compose_file,
            "services:\n  worldserver:\n    environment:\n      TEST: value\n",
        )
        .expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        fs::write(
            playerbots_config,
            "AiPlayerbot.Enabled = 0\nAiPlayerbot.RandomBotAutologin = 0\nAiPlayerbot.MinRandomBots = 0\nAiPlayerbot.MaxRandomBots = 0\nAiPlayerbot.RandomBotGuildCount = 20\n",
        )
        .expect("playerbots config");
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");
        service
            .save_record(&InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: temporary.path().join("game-source"),
                runtime_root: runtime_root.clone(),
                client_executable: client_executable.clone(),
                client_choice: ClientChoice::ManagedOpenWow,
                compose_file,
                bots_enabled: true,
                bot_count: 50,
                ai_enabled: false,
                ai_model: None,
                ollama_executable: None,
                client_sha256: Some("test-openwow-sha256".into()),
                ollama_sha256: None,
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: None,
            })
            .expect("record");

        let status = service.start(None, None, None, |_| {}).expect("start");
        assert_eq!(status.phase, LauncherPhase::Running);
        assert_eq!(service.client_process_id(), Some(42));
        let commands = service.runner.commands.lock().expect("commands");
        assert!(commands.iter().any(|command| {
            command.starts_with(&client_executable.display().to_string())
                && command.contains(&format!("--game-data {}", managed_game.display()))
                && command.contains(&format!("cwd={}", managed_game.display()))
        }));
    }

    #[test]
    fn running_playerbot_population_is_reloaded_without_restarting_the_client() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        let module_config = runtime_root.join("server/env/dist/etc/modules/playerbots.conf.dist");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::create_dir_all(module_config.parent().expect("module config")).expect("module config");
        fs::write(&compose_file, "services: {}").expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        fs::write(
            &module_config,
            "AiPlayerbot.Enabled = 1\nAiPlayerbot.RandomBotAutologin = 1\nAiPlayerbot.MinRandomBots = 5\nAiPlayerbot.MaxRandomBots = 5\nAiPlayerbot.RandomBotGuildCount = 20\n",
        )
        .expect("playerbots config");
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");
        service
            .save_record(&InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: temporary.path().join("game-source"),
                runtime_root: runtime_root.clone(),
                client_executable,
                client_choice: ClientChoice::ManagedOpenWow,
                compose_file,
                bots_enabled: true,
                bot_count: 5,
                ai_enabled: false,
                ai_model: None,
                ollama_executable: None,
                client_sha256: Some("test-openwow-sha256".into()),
                ollama_sha256: None,
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: None,
            })
            .expect("record");
        service.client_process_id = Some(42);

        let status = service
            .update_playerbot_population(true, 100)
            .expect("hot update");
        assert_eq!(status.phase, LauncherPhase::Running);
        assert_eq!(status.bot_count, 100);
        let config =
            fs::read_to_string(runtime_root.join("server/env/dist/etc/modules/playerbots.conf"))
                .expect("updated config");
        assert!(config.contains("AiPlayerbot.MaxRandomBots = 100"));
        let commands = service.runner.commands.lock().expect("commands");
        assert!(commands.iter().any(|command| {
            command.contains("exec -T worldserver sh -lc")
                && command.contains("playerbot rndbot reload")
                && command.contains("playerbot rndbot update")
        }));
        assert!(!commands.iter().any(|command| command == "terminate 42"));
    }

    #[test]
    fn diagnostics_filter_and_redact_runtime_logs() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let logs = temporary.path().join("logs");
        fs::create_dir_all(&logs).expect("logs");
        fs::write(
            logs.join("worldserver.log"),
            "INFO ready\nERROR playerbots failed\nWARNING password=secret\n",
        )
        .expect("log");
        let entries = filtered_log_entries(&logs, 10).expect("entries");
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .any(|entry| entry.contains("playerbots failed"))
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.contains("ligne sensible masquée"))
        );
        assert_eq!(diagnose_entries(&entries, true).0, "bots");
    }
}
