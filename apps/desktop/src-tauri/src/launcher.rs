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

const INSTALL_SCHEMA: u32 = 2;
const RUNTIME_DIRECTORY: &str = "runtime-v2";
const OPENWOW_URL: &str = "https://github.com/rkabachenko/OpenWow-snapshot/releases/download/v0.1.2/OpenWoW-0.1.2-macos-arm64.zip";
const OPENWOW_SHA256: &str = "832cb82fd853417ec64d8fd1a84cb8c6a91a57399fd4b87fb2e810a35b03ed18";
const SERVER_REPOSITORY: &str = "https://github.com/mod-playerbots/azerothcore-wotlk.git";
const SERVER_COMMIT: &str = "47960183bb03b83e8943eb2f0f39c16df9710c9d";
const PLAYERBOTS_REPOSITORY: &str = "https://github.com/mod-playerbots/mod-playerbots.git";
const PLAYERBOTS_COMMIT: &str = "2f7d9f774987d0157c6a0d0cc08c40bec3db3945";
const OLLAMA_CHAT_REPOSITORY: &str = "https://github.com/DustinHendrickson/mod-ollama-chat.git";
const OLLAMA_CHAT_COMMIT: &str = "a9d14b0b8955be136e657ac168dd255f5281a535";
const OLLAMA_URL: &str =
    "https://github.com/ollama/ollama/releases/download/v0.33.2/ollama-darwin.tgz";
const OLLAMA_SHA256: &str = "5751e296a2cd545939bdd51b700de0c20d319f0e723c9d7f48bebb5ab0b731d4";
const OLLAMA_PORT: u16 = 11435;
const MYSQL_IMAGE: &str =
    "mysql@sha256:b3b90af2a6552ae30c266fdb7d5dd55f3afb72404bb78d37fe8a23eb857fd3fb";
const PLAYER_ACCOUNT_NAME: &str = "REALMBOX";
const PLAYER_ACCOUNT_PASSWORD: &str = "REALMBOX";
const SRP6_MODULUS: &str = "894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7";

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
    pub ai_enabled: bool,
    pub ai_model: Option<String>,
    pub game_data_path: Option<String>,
    pub account_name: Option<&'static str>,
    pub account_password: Option<&'static str>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallationRecord {
    schema_version: u32,
    game_data_root: PathBuf,
    runtime_root: PathBuf,
    openwow_executable: PathBuf,
    compose_file: PathBuf,
    bots_enabled: bool,
    ai_enabled: bool,
    ai_model: Option<String>,
    ollama_executable: Option<PathBuf>,
    openwow_sha256: String,
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
    fn spawn(&self, program: &Path, args: &[OsString], log_path: &Path) -> Result<u32, String>;
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

    fn spawn(&self, program: &Path, args: &[OsString], log_path: &Path) -> Result<u32, String> {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let log = File::create(log_path).map_err(|error| error.to_string())?;
        let errors = log.try_clone().map_err(|error| error.to_string())?;
        let child = Command::new(program)
            .args(args)
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(errors))
            .spawn()
            .map_err(|error| format!("impossible de lancer {}: {error}", program.display()))?;
        Ok(child.id())
    }

    fn terminate(&self, process_id: u32) -> Result<(), String> {
        self.run(
            "kill",
            &[
                OsString::from("-TERM"),
                OsString::from(process_id.to_string()),
            ],
            None,
        )
        .map(|_| ())
    }

    fn is_process_running(&self, process_id: u32) -> Result<bool, String> {
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
                if record.openwow_executable.is_file()
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
                ai_enabled: record.ai_enabled,
                ai_model: record.ai_model.clone(),
                game_data_path: Some(record.game_data_root.display().to_string()),
                account_name: None,
                account_password: None,
                components: components(
                    ComponentState::Error,
                    record.bots_enabled,
                    record.ai_enabled,
                    record.ai_model.as_deref(),
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
                ai_enabled: false,
                ai_model: None,
                game_data_path: None,
                account_name: None,
                account_password: None,
                components: components(ComponentState::Error, true, false, None),
            },
        }
    }

    pub fn bootstrap<F>(&mut self, progress: F) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        if self.load_record()?.is_some() {
            self.start(None, None, progress)
        } else {
            Ok(self.status())
        }
    }

    pub fn install<F>(
        &mut self,
        selected_path: &Path,
        bots_enabled: bool,
        ai_enabled: bool,
        ai_model: Option<String>,
        mut progress: F,
    ) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        if cfg!(not(all(target_os = "macos", target_arch = "aarch64"))) {
            return Err("ce premier installateur réel est limité à macOS Apple Silicon".into());
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

        let staging = self.app_data.join(".installing-v2");
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
                "Téléchargement du client OpenWoW",
                Some("Version 0.1.2 officielle"),
            );
            let client_archive = staging.join("OpenWoW-0.1.2-macos-arm64.zip");
            self.runner.run_long(
                "curl",
                &[
                    "-L".into(),
                    "--fail".into(),
                    "--show-error".into(),
                    "--output".into(),
                    client_archive.as_os_str().into(),
                    OPENWOW_URL.into(),
                ],
                None,
                &logs.join("openwow-download.log"),
            )?;
            verify_sha256(&client_archive, OPENWOW_SHA256)?;

            let client_root = staging.join("client");
            fs::create_dir_all(&client_root).map_err(|error| error.to_string())?;
            self.runner.run(
                "ditto",
                &[
                    "-x".into(),
                    "-k".into(),
                    client_archive.as_os_str().into(),
                    client_root.as_os_str().into(),
                ],
                None,
            )?;
            let openwow_executable = client_root.join("OpenWoW.app/Contents/MacOS/openwow-client");
            if !openwow_executable.is_file() {
                return Err("le ZIP OpenWoW vérifié ne contient pas l’exécutable attendu".into());
            }
            self.runner.run(
                "codesign",
                &[
                    "--verify".into(),
                    "--deep".into(),
                    "--strict".into(),
                    client_root.join("OpenWoW.app").as_os_str().into(),
                ],
                None,
            )?;
            fs::remove_file(&client_archive).map_err(|error| error.to_string())?;

            self.emit(
                &mut progress,
                LauncherPhase::Installing,
                22,
                "Téléchargement du serveur épinglé",
                Some("AzerothCore Playerbots"),
            );
            let server_root = staging.join("server");
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

            let uid = self.runner.run("id", &["-u".into()], None)?;
            let gid = self.runner.run("id", &["-g".into()], None)?;
            let database_password = self.runner.run(
                "openssl",
                &["rand".into(), "-hex".into(), "24".into()],
                None,
            )?;
            write_secret_atomic(
                &server_root.join(".env"),
                format!("REALMBOX_DB_PASSWORD={database_password}\n").as_bytes(),
            )?;
            let compose = compose_file(uid.trim(), gid.trim(), &game_data_root);
            let compose_path = server_root.join("compose.realmbox.yaml");
            write_atomic(&compose_path, compose.as_bytes())?;
            write_playerbots_config(&server_root, bots_enabled)?;
            write_ollama_chat_config(&server_root, ai_enabled, ai_model.as_deref())?;

            let staged_ollama = if ai_enabled {
                self.emit(
                    &mut progress,
                    LauncherPhase::Installing,
                    38,
                    "Téléchargement du moteur de dialogue",
                    Some("Ollama 0.33.2 · exécution locale"),
                );
                let archive = staging.join("ollama-darwin-v0.33.2.tgz");
                self.runner.run_long(
                    "curl",
                    &[
                        "-L".into(),
                        "--fail".into(),
                        "--show-error".into(),
                        "--output".into(),
                        archive.as_os_str().into(),
                        OLLAMA_URL.into(),
                    ],
                    None,
                    &logs.join("ollama-download.log"),
                )?;
                verify_sha256(&archive, OLLAMA_SHA256)?;
                let ai_root = staging.join("ai");
                fs::create_dir_all(&ai_root).map_err(|error| error.to_string())?;
                self.runner.run(
                    "tar",
                    &[
                        "-xzf".into(),
                        archive.as_os_str().into(),
                        "-C".into(),
                        ai_root.as_os_str().into(),
                    ],
                    None,
                )?;
                let executable = ai_root.join("ollama");
                let runner_executable = ai_root.join("llama-server");
                if !executable.is_file() || !runner_executable.is_file() {
                    return Err(
                        "l’archive Ollama vérifiée ne contient pas les exécutables attendus".into(),
                    );
                }
                for signed_executable in [&executable, &runner_executable] {
                    self.runner.run(
                        "codesign",
                        &[
                            "--verify".into(),
                            "--strict".into(),
                            signed_executable.as_os_str().into(),
                        ],
                        None,
                    )?;
                }
                fs::remove_file(&archive).map_err(|error| error.to_string())?;
                Some(executable)
            } else {
                None
            };

            let managed_game = staging.join("game");
            prepare_managed_game(&game_data_root, &managed_game, &self.addon_source)?;

            self.emit(
                &mut progress,
                LauncherPhase::Installing,
                42,
                "Construction du serveur local",
                Some("Cette première installation peut être longue"),
            );
            self.runner.run_long(
                "docker",
                &compose_args(
                    &compose_path,
                    &[
                        "build",
                        "db-import",
                        "authserver",
                        "worldserver",
                        "server-data-init",
                    ],
                ),
                Some(&server_root),
                &logs.join("docker-build.log"),
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
                &compose_args(&compose_path, &["up", "-d", "database"]),
                Some(&server_root),
                &logs.join("database-start.log"),
            )?;
            self.runner.wait_tcp(3307, Duration::from_secs(180))?;
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
                openwow_executable: runtime_root
                    .join("client/OpenWoW.app/Contents/MacOS/openwow-client"),
                compose_file: runtime_root.join("server/compose.realmbox.yaml"),
                runtime_root: runtime_root.clone(),
                bots_enabled,
                ai_enabled,
                ai_model: ai_model.clone(),
                ollama_executable: ai_enabled.then(|| runtime_root.join("ai/ollama")),
                openwow_sha256: OPENWOW_SHA256.into(),
                ollama_sha256: ai_enabled.then(|| OLLAMA_SHA256.into()),
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
            let _ = fs::remove_dir_all(staging);
        }
        result
    }

    pub fn start<F>(
        &mut self,
        bots_enabled: Option<bool>,
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
        write_playerbots_config(server_root, record.bots_enabled)?;
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

        self.emit(
            &mut progress,
            LauncherPhase::Starting,
            12,
            "Démarrage de la base locale",
            None,
        );
        self.runner.run_long(
            "docker",
            &compose_args(&record.compose_file, &["up", "-d", "database"]),
            Some(server_root),
            &record.runtime_root.join("logs/start-database.log"),
        )?;
        self.runner.wait_tcp(3307, Duration::from_secs(120))?;

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
        let process_id = self
            .runner
            .spawn(
                &record.openwow_executable,
                &["--game-data".into(), managed_game.as_os_str().into()],
                &record.runtime_root.join("logs/openwow.log"),
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
            ai_enabled: record.ai_enabled,
            ai_model: record.ai_model.clone(),
            game_data_path: Some(record.game_data_root.display().to_string()),
            account_name: Some(PLAYER_ACCOUNT_NAME),
            account_password: Some(PLAYER_ACCOUNT_PASSWORD),
            components: components(
                if running {
                    ComponentState::Running
                } else {
                    ComponentState::Ready
                },
                record.bots_enabled,
                record.ai_enabled,
                record.ai_model.as_deref(),
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
        ai_enabled: false,
        ai_model: None,
        game_data_path: None,
        account_name: None,
        account_password: None,
        components: components(ComponentState::Missing, true, false, None),
    }
}

fn components(
    state: ComponentState,
    bots_enabled: bool,
    ai_enabled: bool,
    ai_model: Option<&str>,
) -> Vec<LauncherComponent> {
    vec![
        LauncherComponent {
            id: "client",
            label: "Client de jeu",
            state,
            detail: "Version locale vérifiée".into(),
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
                "Prêts à peupler le monde".into()
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
                    "{} · calculé sur ce Mac",
                    ai_model.unwrap_or("modèle local")
                )
            } else {
                "Désactivés par le joueur".into()
            },
        },
    ]
}

fn validate_game_data_root(selected: &Path) -> Result<PathBuf, String> {
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
    let locale_found = ["frFR", "enUS", "deDE", "esES", "ruRU"]
        .iter()
        .any(|locale| data.join(locale).is_dir());
    if !locale_found {
        return Err("aucune locale 3.3.5a reconnue n’a été trouvée dans Data".into());
    }
    let has_mpq = fs::read_dir(&data)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("mpq"))
        });
    if !has_mpq {
        return Err("aucune archive MPQ n’a été trouvée à la racine de Data".into());
    }
    fs::canonicalize(root).map_err(|error| error.to_string())
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

fn prepare_managed_game(
    game_data_root: &Path,
    managed_game: &Path,
    addon_source: &Path,
) -> Result<(), String> {
    fs::create_dir_all(managed_game.join("WTF")).map_err(|error| error.to_string())?;
    let addon_destination = managed_game.join("Interface/AddOns/RealmBoxCompanions");
    fs::create_dir_all(&addon_destination).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(game_data_root.join("Data"), managed_game.join("Data"))
        .map_err(|error| error.to_string())?;
    write_atomic(
        &managed_game.join("WTF/Config.wtf"),
        b"SET realmlist \"127.0.0.1\"\nSET portal \"127.0.0.1\"\n",
    )?;
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

fn write_playerbots_config(server_root: &Path, enabled: bool) -> Result<(), String> {
    let value = if enabled { 1 } else { 0 };
    let count = if enabled { 50 } else { 0 };
    let config = format!(
        "AiPlayerbot.Enabled = {value}\nAiPlayerbot.RandomBotAutologin = {value}\nAiPlayerbot.MinRandomBots = {count}\nAiPlayerbot.MaxRandomBots = {count}\n"
    );
    write_atomic(
        &server_root.join("env/dist/etc/playerbots.conf"),
        config.as_bytes(),
    )
}

fn write_ollama_chat_config(
    server_root: &Path,
    enabled: bool,
    model: Option<&str>,
) -> Result<(), String> {
    if enabled && model.is_none() {
        return Err("modèle local absent de la configuration".into());
    }
    if let Some(model) = model
        && !ai::is_allowed_ollama_model(model)
    {
        return Err("modèle Ollama refusé par la liste RealmBox".into());
    }
    let enabled = u8::from(enabled);
    let model = model.unwrap_or("llama3.2:1b");
    let config = format!(
        "[worldserver]\n\
OllamaChat.Enable = {enabled}\n\
OllamaChat.Url = http://host.docker.internal:{OLLAMA_PORT}/api/generate\n\
OllamaChat.Model = {model}\n\
OllamaChat.NumPredict = 72\n\
OllamaChat.ReasoningTokenReserve = 256\n\
OllamaChat.NumCtx = 2048\n\
OllamaChat.MaxConcurrentQueries = 1\n\
OllamaChat.DebugEnabled = 0\n\
OllamaChat.DebugShowFullPrompt = 0\n\
OllamaChat.BotReplyChance.Say = 0\n\
OllamaChat.BotReplyChance.Channel = 0\n\
OllamaChat.BotReplyChance.Party = 0\n\
OllamaChat.BotReplyChance.Guild = 0\n\
OllamaChat.EnableRandomChatter = {enabled}\n\
OllamaChat.RandomChatterBotCommentChance = 2\n\
OllamaChat.RandomChatterMaxBotsPerPlayer = 1\n\
OllamaChat.EnableEventChatter = {enabled}\n\
OllamaChat.EventChatterBotCommentChance = 10\n\
OllamaChat.EventChatterBotSelfCommentChance = 2\n\
OllamaChat.EventChatterMaxBotsPerPlayer = 1\n\
OllamaChat.EnableSentimentTracking = 0\n"
    );
    write_atomic(
        &server_root.join("env/dist/etc/mod_ollama_chat.conf"),
        config.as_bytes(),
    )
}

fn ollama_environment_args(executable: &Path, models: &Path, local_only: bool) -> Vec<OsString> {
    let mut args = vec![
        format!("OLLAMA_HOST=127.0.0.1:{OLLAMA_PORT}").into(),
        format!("OLLAMA_MODELS={}", models.display()).into(),
        "OLLAMA_MAX_LOADED_MODELS=1".into(),
        "OLLAMA_NUM_PARALLEL=1".into(),
        "OLLAMA_MAX_QUEUE=8".into(),
    ];
    if local_only {
        args.push("OLLAMA_NO_CLOUD=true".into());
    }
    args.push(executable.as_os_str().into());
    args
}

fn start_ollama<R: CommandRunner>(
    runner: &R,
    executable: &Path,
    models: &Path,
    log_path: &Path,
    local_only: bool,
) -> Result<u32, String> {
    let mut args = ollama_environment_args(executable, models, local_only);
    args.push("serve".into());
    let process_id = runner.spawn(Path::new("/usr/bin/env"), &args, log_path)?;
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
    let mut args = ollama_environment_args(executable, models, false);
    args.extend(["pull".into(), model.into()]);
    let result = runner.run_long("env", &args, None, &logs.join("ollama-model-download.log"));
    let stop_result = runner.terminate(process_id);
    result.and(stop_result)
}

fn compose_args(compose_file: &Path, trailing: &[&str]) -> Vec<OsString> {
    let mut args = vec![
        "compose".into(),
        "-p".into(),
        "realmbox-v2".into(),
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
    let salt_hex = runner.run(
        "openssl",
        &["rand".into(), "-hex".into(), "32".into()],
        None,
    )?;
    let salt = decode_hex_32(salt_hex.trim())?;
    let verifier = srp6_verifier(PLAYER_ACCOUNT_NAME, PLAYER_ACCOUNT_PASSWORD, &salt)?;
    let sql = format!(
        "INSERT IGNORE INTO account(username,salt,verifier,expansion,reg_mail,email,joindate) VALUES('{PLAYER_ACCOUNT_NAME}',UNHEX('{}'),UNHEX('{}'),2,'','',NOW()); INSERT IGNORE INTO realmcharacters(realmid,acctid,numchars) SELECT realmlist.id,account.id,0 FROM realmlist,account LEFT JOIN realmcharacters ON acctid=account.id WHERE account.username='{PLAYER_ACCOUNT_NAME}' AND acctid IS NULL; UPDATE realmlist SET name='RealmBox',address='127.0.0.1',localAddress='127.0.0.1',port=8085,gamebuild=12340 WHERE id=1;",
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

fn compose_file(uid: &str, gid: &str, game_data_root: &Path) -> String {
    let game_data_mount = serde_json::to_string(&format!(
        "{}:/client-data:ro",
        game_data_root.join("Data").display()
    ))
    .expect("un chemin peut être sérialisé");
    COMPOSE_TEMPLATE
        .replace("__UID__", uid)
        .replace("__GID__", gid)
        .replace("__MYSQL_IMAGE__", MYSQL_IMAGE)
        .replace("__SOURCE_ID__", &source_id(game_data_root))
        .replace("__GAME_DATA_MOUNT__", &game_data_mount)
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
    ports:
      - "127.0.0.1:3307:3306"
    volumes:
      - realmbox-database:/var/lib/mysql
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "127.0.0.1", "-p${REALMBOX_DB_PASSWORD}"]
      interval: 3s
      timeout: 5s
      retries: 60

  server-data-init:
    build:
      context: .
      dockerfile: apps/docker/Dockerfile
      target: tools
      args: { USER_ID: __UID__, GROUP_ID: __GID__, DOCKER_USER: acore }
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
          /azerothcore/env/dist/bin/mmaps_generator;
          rm -f /work/Data;
          echo "REALMBOX_SOURCE_ID=$${REALMBOX_SOURCE_ID}" > /work/extraction-version;
        fi

  db-import:
    build:
      context: .
      dockerfile: apps/docker/Dockerfile
      target: db-import
      args: { USER_ID: __UID__, GROUP_ID: __GID__, DOCKER_USER: acore }
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
    build:
      context: .
      dockerfile: apps/docker/Dockerfile
      target: authserver
      args: { USER_ID: __UID__, GROUP_ID: __GID__, DOCKER_USER: acore }
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
    build:
      context: .
      dockerfile: apps/docker/Dockerfile
      target: worldserver
      args: { USER_ID: __UID__, GROUP_ID: __GID__, DOCKER_USER: acore }
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
        fn spawn(
            &self,
            program: &Path,
            args: &[OsString],
            _log_path: &Path,
        ) -> Result<u32, String> {
            self.commands.lock().expect("commands").push(format!(
                "{} {}",
                program.display(),
                args.iter()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
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
        fn is_process_running(&self, _process_id: u32) -> Result<bool, String> {
            Ok(false)
        }
        fn wait_tcp(&self, _port: u16, _timeout: Duration) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn validates_data_directory_or_its_parent_without_reading_assets() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let root = temporary.path().join("Jeu privé");
        fs::create_dir_all(root.join("Data/frFR")).expect("fixture");
        fs::write(root.join("Data/common.MPQ"), []).expect("fixture");
        assert_eq!(
            validate_game_data_root(&root).expect("root"),
            fs::canonicalize(&root).expect("canonical")
        );
        assert_eq!(
            validate_game_data_root(&root.join("Data")).expect("data"),
            fs::canonicalize(&root).expect("canonical")
        );
    }

    #[test]
    fn compose_pins_database_and_server_data_and_binds_ports_locally() {
        let compose = compose_file("501", "20", Path::new("/Jeux privés/Wrath"));
        assert!(compose.contains(MYSQL_IMAGE));
        assert!(compose.contains(r#"/Jeux privés/Wrath/Data:/client-data:ro"#));
        assert!(compose.contains("map_extractor"));
        assert!(compose.contains("mmaps_generator"));
        assert!(compose.contains("127.0.0.1:3724:3724"));
        assert!(compose.contains("127.0.0.1:8085:8085"));
        assert!(compose.contains("host.docker.internal:host-gateway"));
        assert!(!compose.contains("image: mysql:8.4"));
    }

    #[test]
    fn ollama_chat_is_local_bounded_and_allowlisted() {
        let temporary = tempfile::tempdir().expect("tempdir");
        write_ollama_chat_config(temporary.path(), true, Some("qwen3:8b")).expect("valid model");
        let config = fs::read_to_string(temporary.path().join("env/dist/etc/mod_ollama_chat.conf"))
            .expect("config");
        assert!(config.contains("http://host.docker.internal:11435/api/generate"));
        assert!(config.contains("OllamaChat.MaxConcurrentQueries = 1"));
        assert!(config.contains("OllamaChat.BotReplyChance.Say = 0"));
        let runtime_args = ollama_environment_args(
            Path::new("/managed/ai/ollama"),
            Path::new("/managed/ai/models"),
            true,
        );
        assert!(runtime_args.contains(&OsString::from("OLLAMA_NO_CLOUD=true")));
        assert!(runtime_args.contains(&OsString::from("OLLAMA_MAX_QUEUE=8")));
        assert!(
            write_ollama_chat_config(temporary.path(), true, Some("remote.example/model:latest"))
                .is_err()
        );
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
        assert!(commands[1].contains("docker compose -p realmbox"));
        assert!(commands[1].contains("INSERT IGNORE INTO account"));
        assert!(commands[1].contains("UPDATE realmlist SET name='RealmBox'"));
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
        let args = compose_args(path, &["up", "-d", "database"]);
        assert_eq!(args[0], "compose");
        assert_eq!(args[1], "-p");
        assert_eq!(args[2], "realmbox-v2");
        assert_eq!(args.last(), Some(&OsString::from("database")));
        let runner = RecordingRunner::default();
        runner.run("docker", &args, None).expect("recorded");
        assert!(
            runner.commands.lock().expect("commands")[0]
                .starts_with("docker compose -p realmbox-v2")
        );
    }

    #[test]
    fn client_exit_stops_the_owned_world_and_ai_process() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join("runtime-v2");
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let openwow_executable = runtime_root.join("client/openwow-client");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(openwow_executable.parent().expect("client")).expect("client");
        fs::write(&compose_file, "services: {}").expect("compose");
        fs::write(&openwow_executable, "binary").expect("client");
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
                openwow_executable,
                compose_file,
                bots_enabled: true,
                ai_enabled: true,
                ai_model: Some("qwen3:8b".into()),
                ollama_executable: Some(temporary.path().join("ai/ollama")),
                openwow_sha256: OPENWOW_SHA256.into(),
                ollama_sha256: Some(OLLAMA_SHA256.into()),
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
}
