use std::{
    collections::HashMap,
    env,
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::Mutex,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};

use crate::{
    ai::{self, AiCapability},
    local_guide::{
        HexEncodedGuideTerm, LocalGuideLocale, LocalGuideQuery, LocalGuideResponse,
        LocalGuideSearch, LocalGuideSearchDataSource, LocalGuideTabularSnapshot, LocalProvenance,
        LocalSourceError, LocalSourceScope,
    },
    solo_profile_store::{SoloProfileStore, SoloProfileView},
    solo_profiles::SoloProfile,
};

pub(crate) const INSTALL_SCHEMA: u32 = 3;
const RUNTIME_DIRECTORY: &str = "runtime-v3";
const COMPOSE_PROJECT_NAME: &str = "realmbox-v3";
const PLAYER_DATA_BACKUP_DIRECTORY: &str = "player-data-backups";
const RUNTIME_ROLLBACK_DIRECTORY: &str = "runtime-rollbacks";
const RUNTIME_UPDATE_FILE: &str = "runtime-update.json";
const DOCKER_RECOVERY_FILE: &str = "docker-recovery.json";
const DATABASE_VOLUME: &str = "realmbox-v3_realmbox-database";
const SERVER_DATA_VOLUME: &str = "realmbox-v3_realmbox-server-data";
const SERVER_REPOSITORY: &str = "https://github.com/mod-playerbots/azerothcore-wotlk.git";
const SERVER_COMMIT: &str = "47960183bb03b83e8943eb2f0f39c16df9710c9d";
const PLAYERBOTS_REPOSITORY: &str = "https://github.com/mod-playerbots/mod-playerbots.git";
const PLAYERBOTS_COMMIT: &str = "2f7d9f774987d0157c6a0d0cc08c40bec3db3945";
const OLLAMA_CHAT_REPOSITORY: &str = "https://github.com/DustinHendrickson/mod-ollama-chat.git";
const OLLAMA_CHAT_COMMIT: &str = "a9d14b0b8955be136e657ac168dd255f5281a535";
const REALMBOX_PRESENCE_MODULE: &str = "mod-realmbox-presence";
const REALMBOX_OLLAMA_PATCH: &str = "mod-ollama-chat-realmbox.patch";
const OLLAMA_PORT: u16 = 11435;
const LOCAL_GUIDE_SHORT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const LOCAL_GUIDE_DATABASE_START_TIMEOUT: Duration = Duration::from_secs(125);
const LOCAL_GUIDE_QUERY_TIMEOUT: Duration = Duration::from_secs(10);
const LOCAL_GUIDE_DATABASE_STOP_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_BOUNDED_COMMAND_OUTPUT_BYTES: u64 = 64 * 1_024;
const LOCAL_GUIDE_QUEST_SQL: &str = r#"SET NAMES utf8mb4; START TRANSACTION READ ONLY; SELECT /*+ MAX_EXECUTION_TIME(2000) */ q.ID, HEX(LEFT(COALESCE(NULLIF(l.Title, ''), q.LogTitle, ''), 121)), HEX(LEFT(COALESCE(NULLIF(l.Details, ''), q.QuestDescription, q.LogDescription, ''), 321)), IF(q.QuestLevel BETWEEN 0 AND 1000, q.QuestLevel, '-'), HEX('quest') FROM quest_template q LEFT JOIN quest_template_locale l ON l.ID = q.ID AND l.locale = '__LOCALE__' WHERE LOWER(COALESCE(NULLIF(l.Title, ''), q.LogTitle, '')) LIKE CONCAT('%', LOWER(CONVERT(UNHEX('$REALMBOX_GUIDE_TERM_HEX') USING utf8mb4)), '%') ORDER BY q.QuestLevel, q.ID LIMIT 8; COMMIT;"#;
const LOCAL_GUIDE_ITEM_SQL: &str = r#"SET NAMES utf8mb4; START TRANSACTION READ ONLY; SELECT /*+ MAX_EXECUTION_TIME(2000) */ i.entry, HEX(LEFT(COALESCE(NULLIF(l.Name, ''), i.name, ''), 121)), HEX(LEFT(COALESCE(NULLIF(l.Description, ''), i.description, ''), 321)), i.RequiredLevel, HEX(CONCAT('iLevel ', i.ItemLevel)) FROM item_template i LEFT JOIN item_template_locale l ON l.ID = i.entry AND l.locale = '__LOCALE__' WHERE LOWER(COALESCE(NULLIF(l.Name, ''), i.name, '')) LIKE CONCAT('%', LOWER(CONVERT(UNHEX('$REALMBOX_GUIDE_TERM_HEX') USING utf8mb4)), '%') ORDER BY i.RequiredLevel, i.entry LIMIT 8; COMMIT;"#;
const OLLAMA_DIALOGUE_SYSTEM_PROMPT_EN: &str = r#""Reply directly in exactly the language of the quoted player message. An English message requires an English answer. A French message requires a French answer. If there is no quoted player message, write only in English. Keep names and World of Warcraft terms unchanged. If unsure, say so briefly in the required language. Output only the answer.""#;
const OLLAMA_DIALOGUE_SYSTEM_PROMPT_FR: &str = r#""Reply directly in exactly the language of the quoted player message. An English message requires an English answer. A French message requires a French answer. If there is no quoted player message, write only in French. Keep names and World of Warcraft terms unchanged. If unsure, say so briefly in the required language. Output only the answer.""#;
const OLLAMA_DIALOGUE_CHAT_PROMPT: &str = r#""You are {bot_name}, a World of Warcraft {bot_class}. Player message: <player_message>{player_message}</player_message>. Reply directly to that message in the same language in under 20 words. Output only the answer, with no name, prefix, narration, classification, or meta-comment.""#;
const OLLAMA_RANDOM_PROMPT_EN: &str = r#""You are {bot_name}, a level {bot_level} {bot_class} in {bot_area}, {bot_zone}. Personality: {bot_personality_name}: {bot_personality}. {environment_info} Write one natural in-character World of Warcraft remark in English, under 15 words. No name, prefix, quote, emoji, markdown, narration, or meta-comment.""#;
const OLLAMA_RANDOM_VARIATIONS_EN: &str = r#""Comment on the current place or journey.|Mention a quest, fight, profession, equipment, or group need that fits the situation.|Make a brief observation about your class or role.|React to the atmosphere without inventing an event.""#;
const OLLAMA_EVENT_PROMPT_EN: &str = r#""You are {bot_name}, a level {bot_level} {bot_class} in {bot_area}, {bot_zone}. Personality: {bot_personality_name}: {bot_personality}. Event: {actor_name} {event_type} {event_detail}. React only to this event in natural English, in character, under 15 words. No name, prefix, emoji, markdown, narration, or invented facts.""#;
const OLLAMA_RANDOM_PROMPT_FR: &str = r#""Tu es {bot_name}, {bot_class} de niveau {bot_level}, à {bot_area}, {bot_zone}. Personnalité : {bot_personality_name}: {bot_personality}. {environment_info} Écris une seule remarque naturelle et crédible en français, dans le monde de World of Warcraft, en moins de 15 mots. Aucun nom, préfixe, guillemet, emoji, markdown, récit ou méta-commentaire.""#;
const OLLAMA_RANDOM_VARIATIONS_FR: &str = r#""Commente le lieu actuel ou le voyage.|Mentionne une quête, un combat, un métier, un équipement ou un besoin de groupe adapté à la situation.|Fais une brève remarque sur ta classe ou ton rôle.|Réagis à l’ambiance sans inventer d’événement.""#;
const OLLAMA_EVENT_PROMPT_FR: &str = r#""Tu es {bot_name}, {bot_class} de niveau {bot_level}, à {bot_area}, {bot_zone}. Personnalité : {bot_personality_name}: {bot_personality}. Événement : {actor_name} {event_type} {event_detail}. Réagis uniquement à cet événement, naturellement en français et dans ton rôle, en moins de 15 mots. Aucun nom, préfixe, emoji, markdown, récit ou fait inventé.""#;
const MYSQL_IMAGE: &str =
    "mysql@sha256:b3b90af2a6552ae30c266fdb7d5dd55f3afb72404bb78d37fe8a23eb857fd3fb";
const AUTH_SERVER_IMAGE: Option<&str> = option_env!("REALMBOX_AUTH_SERVER_IMAGE");
const WORLD_SERVER_IMAGE: Option<&str> = option_env!("REALMBOX_WORLD_SERVER_IMAGE");
const DB_IMPORT_IMAGE: Option<&str> = option_env!("REALMBOX_DB_IMPORT_IMAGE");
const TOOLS_IMAGE: Option<&str> = option_env!("REALMBOX_TOOLS_IMAGE");
const DEFAULT_DOCKER_BUILD_JOBS: usize = 2;
const BASE_INSTALLATION_DISK_BYTES: u64 = 24 * 1024 * 1024 * 1024;
const DISK_SAFETY_MARGIN_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const PLAYER_ACCOUNT_NAME: &str = "REALMBOX";
const PLAYER_ACCOUNT_PASSWORD: &str = "REALMBOX";
const SRP6_MODULUS: &str = "894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7";
const SUPPORTED_GAME_LOCALES: [&str; 10] = [
    "frFR", "enUS", "enGB", "deDE", "esES", "esMX", "ruRU", "koKR", "zhCN", "zhTW",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DialogueChattiness {
    Quiet,
    #[default]
    Balanced,
    Lively,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum DialogueLanguage {
    French,
    #[default]
    English,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BotPresence {
    Dispersed,
    #[default]
    Natural,
    Close,
}

fn legacy_bot_presence() -> BotPresence {
    BotPresence::Close
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
    Recovering,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationComponent {
    Launcher,
    GameData,
    Client,
    Server,
    Database,
    Bots,
    Ai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationStep {
    Validate,
    Download,
    Verify,
    Extract,
    Configure,
    Start,
    Stop,
    Restore,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    DockerMissing,
    DockerNotRunning,
    PortUnavailable,
    GameDataIncomplete,
    GameBuildUnsupported,
    DownloadInterrupted,
    ChecksumMismatch,
    BackupFailed,
    MigrationFailed,
    RecoveryFailed,
    ClientLaunchFailed,
    WorldServerTimeout,
    InstallationIncomplete,
    InstallationStateUnreadable,
    OperationUnavailable,
}

impl ErrorCode {
    pub fn from_detail(detail: &str, fallback: Self) -> Self {
        let detail = detail.to_lowercase();
        if detail.contains("docker desktop doit être installé") {
            Self::DockerMissing
        } else if detail.contains("docker desktop doit être démarré")
            || detail.contains("docker desktop doit être installé et démarré")
        {
            Self::DockerNotRunning
        } else if detail.contains("port")
            && (detail.contains("occup") || detail.contains("indisponible"))
        {
            Self::PortUnavailable
        } else if detail.contains("mpq")
            || detail.contains("archive wotlk")
            || detail.contains("archive de locale")
        {
            Self::GameDataIncomplete
        } else if detail.contains("build 12340") || detail.contains("version du client") {
            Self::GameBuildUnsupported
        } else if detail.contains("sha-256")
            || detail.contains("checksum")
            || detail.contains("empreinte")
        {
            Self::ChecksumMismatch
        } else if detail.contains("sauvegarde") || detail.contains("dump") {
            Self::BackupFailed
        } else if detail.contains("migration") || detail.contains("db-import") {
            Self::MigrationFailed
        } else if detail.contains("restaur") || detail.contains("récupération") {
            Self::RecoveryFailed
        } else if detail.contains("télécharg")
            || detail.contains("curl")
            || detail.contains("download")
        {
            Self::DownloadInterrupted
        } else if detail.contains("openwow")
            || detail.contains("wow.exe")
            || detail.contains("client")
        {
            Self::ClientLaunchFailed
        } else if detail.contains("worldserver") || detail.contains("serveur") {
            Self::WorldServerTimeout
        } else {
            fallback
        }
    }
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
    pub error_code: Option<ErrorCode>,
    pub progress: u8,
    pub installed: bool,
    pub recovery_available: bool,
    pub bots_enabled: bool,
    pub bot_count: usize,
    pub requested_bot_count: usize,
    pub applied_bot_count: usize,
    pub bot_presence: BotPresence,
    pub ai_enabled: bool,
    pub ai_model: Option<String>,
    pub dialogue_chattiness: DialogueChattiness,
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
    pub operation_id: String,
    pub component: OperationComponent,
    pub step: OperationStep,
    pub phase: LauncherPhase,
    pub message: String,
    pub detail: Option<String>,
    pub error_code: Option<ErrorCode>,
    pub progress: u8,
    pub completed_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub cancellable: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmBackupSummary {
    pub created_at_unix_ms: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationOptions {
    pub client_choice: ClientChoice,
    pub bots_enabled: bool,
    pub bot_count: usize,
    pub bot_presence: BotPresence,
    pub ai_enabled: bool,
    pub ai_model: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldPreferences {
    bots_enabled: bool,
    requested_bot_count: usize,
    #[serde(default = "legacy_bot_presence")]
    bot_presence: BotPresence,
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
    #[serde(default)]
    runtime_release: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryPoint {
    stem: String,
    backup: PathBuf,
    rollback_root: PathBuf,
    rollback_server: PathBuf,
    metadata: Option<RecoveryMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryMetadata {
    schema_version: u32,
    stem: String,
    source_runtime_release: Option<String>,
    target_runtime_release: String,
    ai_enabled: bool,
    ai_model: Option<String>,
    ollama_chat_commit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RuntimeUpdatePhase {
    Staged,
    Published,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeUpdateTransaction {
    schema_version: u32,
    transition: String,
    attempt: u32,
    phase: RuntimeUpdatePhase,
    images: ServerImages,
    recovery: RecoveryMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatabaseBackup {
    stem: String,
    backup: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DockerRecoveryRecord {
    schema_version: u32,
    backup_stem: String,
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
    fn run_bounded(
        &self,
        _program: &str,
        _args: &[OsString],
        _current_dir: Option<&Path>,
        _timeout: Duration,
    ) -> Result<String, String> {
        Err("exécution bornée non prise en charge par ce runner".into())
    }
    fn run_long_bounded(
        &self,
        _program: &str,
        _args: &[OsString],
        _current_dir: Option<&Path>,
        _log_path: &Path,
        _timeout: Duration,
    ) -> Result<(), String> {
        Err("exécution bornée non prise en charge par ce runner".into())
    }
    fn run_to_file(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
        output_path: &Path,
        _error_path: &Path,
    ) -> Result<(), String> {
        self.run_long(program, args, current_dir, output_path)
    }
    fn run_with_input(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
        input_path: &Path,
        log_path: &Path,
    ) -> Result<(), String> {
        let _ = input_path;
        self.run_long(program, args, current_dir, log_path)
    }
    fn download_verified(
        &self,
        url: &str,
        expected_sha256: &str,
        destination: &Path,
        log_path: &Path,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), String> {
        self.run_long(
            "curl",
            &[
                "-L".into(),
                "--fail".into(),
                "--show-error".into(),
                "--output".into(),
                destination.as_os_str().into(),
                url.into(),
            ],
            None,
            log_path,
        )?;
        let total = fs::metadata(destination)
            .map(|metadata| metadata.len())
            .ok();
        progress(total.unwrap_or(0), total);
        verify_sha256(destination, expected_sha256)
    }
    fn post_json(&self, url: &str, body: &str, timeout: Duration) -> Result<String, String> {
        self.run(
            "curl",
            &[
                "--silent".into(),
                "--show-error".into(),
                "--fail".into(),
                "--max-time".into(),
                timeout.as_secs().to_string().into(),
                "--request".into(),
                "POST".into(),
                url.into(),
                "--header".into(),
                "content-type: application/json".into(),
                "--data".into(),
                body.into(),
            ],
            None,
        )
    }
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
    fn wait_service_tcp(
        &self,
        compose_file: &Path,
        service: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<(), String>;
    fn wait_tcp(&self, port: u16, timeout: Duration) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct SystemCommandRunner {
    owned_processes: Mutex<HashMap<u32, Child>>,
    #[cfg(windows)]
    job_handles: Mutex<HashMap<u32, isize>>,
}

struct BoundedCommandCapture {
    path: PathBuf,
    file: Option<File>,
}

impl BoundedCommandCapture {
    fn new() -> Result<Self, String> {
        let path = env::temp_dir().join(format!(
            "realmbox-command-{}.capture",
            secure_random_hex(16)?
        ));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(&path)
            .map_err(|error| format!("capture de commande impossible: {error}"))?;
        Ok(Self {
            path,
            file: Some(file),
        })
    }

    fn writer(&self) -> Result<File, String> {
        self.file
            .as_ref()
            .ok_or_else(|| "capture de commande fermée".to_string())?
            .try_clone()
            .map_err(|error| format!("capture de commande impossible: {error}"))
    }

    fn read_bounded(&mut self) -> Result<Vec<u8>, String> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| "capture de commande fermée".to_string())?;
        if file.metadata().map_err(|error| error.to_string())?.len()
            > MAX_BOUNDED_COMMAND_OUTPUT_BYTES
        {
            return Err("sortie de commande supérieure à 64 KiB".into());
        }
        file.seek(SeekFrom::Start(0))
            .map_err(|error| error.to_string())?;
        let mut bytes = Vec::new();
        file.take(MAX_BOUNDED_COMMAND_OUTPUT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("capture de commande illisible: {error}"))?;
        if bytes.len() as u64 > MAX_BOUNDED_COMMAND_OUTPUT_BYTES {
            return Err("sortie de commande supérieure à 64 KiB".into());
        }
        Ok(bytes)
    }
}

impl Drop for BoundedCommandCapture {
    fn drop(&mut self) {
        self.file.take();
        let _ = fs::remove_file(&self.path);
    }
}

impl SystemCommandRunner {
    #[cfg(windows)]
    fn assign_job_object(
        &self,
        process_id: u32,
        child: &std::process::Child,
    ) -> Result<(), String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
        };

        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err(format!(
                "création du Job Object Windows impossible: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            unsafe { CloseHandle(job) };
            return Err(format!(
                "configuration du Job Object Windows impossible: {}",
                std::io::Error::last_os_error()
            ));
        }
        let assigned = unsafe { AssignProcessToJobObject(job, child.as_raw_handle().cast()) };
        if assigned == 0 {
            unsafe { CloseHandle(job) };
            return Err(format!(
                "association du processus au Job Object Windows impossible: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut handles = match self.job_handles.lock() {
            Ok(handles) => handles,
            Err(_) => {
                unsafe { CloseHandle(job) };
                return Err("registre des Job Objects indisponible".to_string());
            }
        };
        handles.insert(process_id, job as isize);
        Ok(())
    }

    #[cfg(windows)]
    fn windows_job_is_running(&self, process_id: u32) -> Result<bool, String> {
        use windows_sys::Win32::System::JobObjects::{
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JobObjectBasicAccountingInformation,
            QueryInformationJobObject,
        };
        let handle = self
            .job_handles
            .lock()
            .map_err(|_| "registre des Job Objects indisponible".to_string())?
            .get(&process_id)
            .copied()
            .ok_or_else(|| "processus non possédé par RealmBox".to_string())?;
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        let queried = unsafe {
            QueryInformationJobObject(
                handle as _,
                JobObjectBasicAccountingInformation,
                (&raw mut accounting).cast(),
                std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                std::ptr::null_mut(),
            )
        };
        if queried == 0 {
            return Err(format!(
                "inspection du Job Object Windows impossible: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(accounting.ActiveProcesses > 0)
    }

    #[cfg(windows)]
    fn close_windows_job(&self, process_id: u32) -> Result<(), String> {
        use windows_sys::Win32::Foundation::CloseHandle;
        let handle = self
            .job_handles
            .lock()
            .map_err(|_| "registre des Job Objects indisponible".to_string())?
            .remove(&process_id)
            .ok_or_else(|| "Job Object du processus introuvable".to_string())?;
        if unsafe { CloseHandle(handle as _) } == 0 {
            return Err(format!(
                "fermeture du Job Object Windows impossible: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    fn spawn_owned_command(
        &self,
        command: &mut Command,
        display_name: &str,
    ) -> Result<u32, String> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .spawn()
            .map_err(|error| format!("impossible de lancer {display_name}: {error}"))?;
        let process_id = child.id();
        #[cfg(windows)]
        if let Err(error) = self.assign_job_object(process_id, &child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        let mut owned_processes = match self.owned_processes.lock() {
            Ok(processes) => processes,
            Err(_) => {
                #[cfg(windows)]
                let _ = self.close_windows_job(process_id);
                #[cfg(unix)]
                let _ = Command::new("kill")
                    .args(["-KILL", &format!("-{process_id}")])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                let _ = child.kill();
                let _ = child.wait();
                return Err("registre des processus possédés indisponible".to_string());
            }
        };
        if owned_processes.contains_key(&process_id) {
            #[cfg(windows)]
            let _ = self.close_windows_job(process_id);
            #[cfg(unix)]
            let _ = Command::new("kill")
                .args(["-KILL", &format!("-{process_id}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = child.kill();
            let _ = child.wait();
            return Err("collision d’identifiant de processus possédé".into());
        }
        owned_processes.insert(process_id, child);
        Ok(process_id)
    }

    fn force_terminate_owned_command(&self, process_id: u32) -> Result<(), String> {
        let mut child = self
            .owned_processes
            .lock()
            .map_err(|_| "registre des processus possédés indisponible".to_string())?
            .remove(&process_id)
            .ok_or_else(|| "processus borné non possédé par RealmBox".to_string())?;

        #[cfg(unix)]
        {
            let already_exited = child
                .try_wait()
                .map_err(|error| format!("inspection du processus borné impossible: {error}"))?
                .is_some();
            let group_result = Command::new("kill")
                .args(["-KILL", &format!("-{process_id}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|error| format!("arrêt du groupe de processus borné impossible: {error}"));
            let direct_result = if already_exited { Ok(()) } else { child.kill() };
            child
                .wait()
                .map_err(|error| format!("attente du processus borné impossible: {error}"))?;
            if !already_exited {
                if let Ok(status) = &group_result
                    && !status.success()
                    && direct_result.is_err()
                {
                    return Err("arrêt du processus borné impossible".into());
                }
                group_result?;
            }
            return Ok(());
        }

        #[cfg(windows)]
        {
            let job_result = self.close_windows_job(process_id);
            if job_result.is_err() {
                let _ = child.kill();
            }
            child
                .wait()
                .map_err(|error| format!("attente du processus borné impossible: {error}"))?;
            return job_result;
        }

        #[allow(unreachable_code)]
        Err("arrêt de processus borné non pris en charge sur cette plateforme".into())
    }

    fn wait_owned_command(
        &self,
        process_id: u32,
        display_name: &str,
        timeout: Duration,
    ) -> Result<ExitStatus, String> {
        let started = Instant::now();
        loop {
            let status_result = {
                let mut processes = self
                    .owned_processes
                    .lock()
                    .map_err(|_| "registre des processus possédés indisponible".to_string())?;
                let child = processes
                    .get_mut(&process_id)
                    .ok_or_else(|| "processus borné non possédé par RealmBox".to_string())?;
                child
                    .try_wait()
                    .map_err(|error| format!("inspection de {display_name} impossible: {error}"))
            };
            let status = match status_result {
                Ok(status) => status,
                Err(error) => {
                    let _ = self.force_terminate_owned_command(process_id);
                    return Err(error);
                }
            };
            if let Some(status) = status {
                // Close the whole owned group/job before reading capture files:
                // a descendant must not outlive a completed command.
                self.force_terminate_owned_command(process_id)?;
                return Ok(status);
            }
            let elapsed = started.elapsed();
            if elapsed >= timeout {
                let cleanup = self.force_terminate_owned_command(process_id);
                let timeout_ms = timeout.as_millis();
                return Err(match cleanup {
                    Ok(()) => format!(
                        "{display_name} a dépassé le délai de {timeout_ms} ms et a été arrêté"
                    ),
                    Err(error) => format!(
                        "{display_name} a dépassé le délai de {timeout_ms} ms ; arrêt à vérifier : {error}"
                    ),
                });
            }
            thread::sleep(
                timeout
                    .saturating_sub(elapsed)
                    .min(Duration::from_millis(20)),
            );
        }
    }
}

#[cfg(windows)]
impl Drop for SystemCommandRunner {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        if let Ok(handles) = self.job_handles.get_mut() {
            for (_, handle) in handles.drain() {
                unsafe { CloseHandle(handle as _) };
            }
        }
    }
}

#[cfg(unix)]
impl Drop for SystemCommandRunner {
    fn drop(&mut self) {
        if let Ok(processes) = self.owned_processes.get_mut() {
            for (process_id, mut child) in processes.drain() {
                let _ = Command::new("kill")
                    .args(["-TERM", &format!("-{process_id}")])
                    .output();
                let _ = child.wait();
            }
        }
    }
}

fn resolve_program(program: &str) -> PathBuf {
    resolve_program_with(
        program,
        env::var_os("PATH").as_deref(),
        &docker_desktop_cli_candidates(),
    )
}

fn resolve_program_with(
    program: &str,
    search_path: Option<&OsStr>,
    docker_fallbacks: &[PathBuf],
) -> PathBuf {
    let requested = PathBuf::from(program);
    if requested.components().count() > 1 {
        return requested;
    }

    if let Some(search_path) = search_path {
        for directory in env::split_paths(search_path) {
            for executable_name in executable_names(program) {
                let candidate = directory.join(executable_name);
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }

    if program == "docker"
        && let Some(candidate) = docker_fallbacks.iter().find(|path| path.is_file())
    {
        return candidate.clone();
    }

    requested
}

fn executable_names(program: &str) -> Vec<OsString> {
    #[cfg(windows)]
    {
        let requested = OsString::from(program);
        if Path::new(program).extension().is_some() {
            vec![requested]
        } else {
            vec![requested, OsString::from(format!("{program}.exe"))]
        }
    }

    #[cfg(not(windows))]
    {
        vec![OsString::from(program)]
    }
}

fn docker_desktop_cli_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return vec![
            PathBuf::from("/usr/local/bin/docker"),
            PathBuf::from("/opt/homebrew/bin/docker"),
            PathBuf::from("/Applications/Docker.app/Contents/Resources/bin/docker"),
        ];
    }

    #[cfg(target_os = "windows")]
    {
        return [env::var_os("ProgramW6432"), env::var_os("ProgramFiles")]
            .into_iter()
            .flatten()
            .map(PathBuf::from)
            .map(|root| root.join("Docker/Docker/resources/bin/docker.exe"))
            .collect();
    }

    #[allow(unreachable_code)]
    Vec::new()
}

fn child_command(program: &str) -> Command {
    let resolved = resolve_program(program);
    let mut command = Command::new(&resolved);
    if program == "docker"
        && let Some(path) = docker_support_path(
            env::var_os("PATH").as_deref(),
            &resolved,
            &docker_desktop_cli_candidates(),
        )
    {
        command.env("PATH", path);
    }
    command
}

fn docker_support_path(
    current: Option<&OsStr>,
    resolved: &Path,
    candidates: &[PathBuf],
) -> Option<OsString> {
    let mut directories = Vec::new();
    let mut append = |directory: PathBuf| {
        if !directory.as_os_str().is_empty() && !directories.contains(&directory) {
            directories.push(directory);
        }
    };
    if let Some(parent) = resolved.parent() {
        append(parent.to_path_buf());
    }
    for directory in candidates
        .iter()
        .filter_map(|candidate| candidate.parent().map(Path::to_path_buf))
    {
        append(directory);
    }
    if let Some(current) = current {
        for directory in env::split_paths(current) {
            append(directory);
        }
    }
    env::join_paths(directories).ok()
}

impl CommandRunner for SystemCommandRunner {
    fn run(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
    ) -> Result<String, String> {
        let mut command = child_command(program);
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

    fn run_bounded(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
        timeout: Duration,
    ) -> Result<String, String> {
        let mut stdout_capture = BoundedCommandCapture::new()?;
        let mut stderr_capture = BoundedCommandCapture::new()?;
        let mut command = child_command(program);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_capture.writer()?))
            .stderr(Stdio::from(stderr_capture.writer()?));
        if let Some(directory) = current_dir {
            command.current_dir(directory);
        }
        let process_id = self.spawn_owned_command(&mut command, program)?;
        drop(command);
        let status = self.wait_owned_command(process_id, program, timeout)?;
        let stdout = stdout_capture.read_bounded()?;
        let stderr = stderr_capture.read_bounded()?;
        if !status.success() {
            return Err(format!(
                "{program} a échoué ({status}): {}",
                String::from_utf8_lossy(&stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&stdout).trim().to_string())
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
        let mut command = child_command(program);
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

    fn run_long_bounded(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
        log_path: &Path,
        timeout: Duration,
    ) -> Result<(), String> {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let log = File::create(log_path).map_err(|error| error.to_string())?;
        let errors = log.try_clone().map_err(|error| error.to_string())?;
        let mut command = child_command(program);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(errors));
        if let Some(directory) = current_dir {
            command.current_dir(directory);
        }
        let process_id = self.spawn_owned_command(&mut command, program)?;
        drop(command);
        let status = self.wait_owned_command(process_id, program, timeout)?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "{program} a échoué ({status}); voir {}",
                log_path.display()
            ))
        }
    }

    fn run_to_file(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
        output_path: &Path,
        error_path: &Path,
    ) -> Result<(), String> {
        for path in [output_path, error_path] {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
        }
        let output = File::create(output_path).map_err(|error| error.to_string())?;
        let errors = File::create(error_path).map_err(|error| error.to_string())?;
        let mut command = child_command(program);
        command
            .args(args)
            .stdout(Stdio::from(output))
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
                error_path.display()
            ))
        }
    }

    fn run_with_input(
        &self,
        program: &str,
        args: &[OsString],
        current_dir: Option<&Path>,
        input_path: &Path,
        log_path: &Path,
    ) -> Result<(), String> {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let input = File::open(input_path).map_err(|error| error.to_string())?;
        let log = File::create(log_path).map_err(|error| error.to_string())?;
        let errors = log.try_clone().map_err(|error| error.to_string())?;
        let mut command = child_command(program);
        command
            .args(args)
            .stdin(Stdio::from(input))
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

    fn download_verified(
        &self,
        url: &str,
        expected_sha256: &str,
        destination: &Path,
        log_path: &Path,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), String> {
        download_verified_http(url, expected_sha256, destination, log_path, progress)
    }

    fn post_json(&self, url: &str, body: &str, timeout: Duration) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(timeout.min(Duration::from_secs(5)))
            .timeout(timeout)
            .build()
            .map_err(|error| format!("initialisation HTTP impossible: {error}"))?;
        let response = client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_owned())
            .send()
            .map_err(|error| format!("requête HTTP impossible: {error}"))?
            .error_for_status()
            .map_err(|error| format!("réponse HTTP refusée: {error}"))?;
        response
            .text()
            .map_err(|error| format!("réponse HTTP illisible: {error}"))
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
        let mut command = Command::new(program);
        command
            .args(args)
            .envs(environment.iter().cloned())
            .current_dir(current_dir.unwrap_or_else(|| Path::new(".")))
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(errors));
        self.spawn_owned_command(&mut command, &program.display().to_string())
    }

    fn terminate(&self, process_id: u32) -> Result<(), String> {
        let mut child = self
            .owned_processes
            .lock()
            .map_err(|_| "registre des processus possédés indisponible".to_string())?
            .remove(&process_id)
            .ok_or_else(|| {
                "RealmBox refuse d’arrêter un processus qu’il n’a pas créé".to_string()
            })?;

        #[cfg(unix)]
        {
            let already_exited = child
                .try_wait()
                .map_err(|error| format!("inspection du processus impossible: {error}"))?
                .is_some();
            let result = Command::new("kill")
                .args(["-TERM", &format!("-{process_id}")])
                .output()
                .map_err(|error| format!("arrêt du groupe de processus impossible: {error}"));
            if let Ok(output) = &result
                && !output.status.success()
                && !already_exited
            {
                return Err(format!(
                    "arrêt du groupe de processus impossible: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            result?;
            if !already_exited {
                child
                    .wait()
                    .map_err(|error| format!("attente du processus impossible: {error}"))?;
            }
            return Ok(());
        }

        #[cfg(windows)]
        {
            let result = self.close_windows_job(process_id);
            let _ = child.wait();
            return result;
        }

        #[allow(unreachable_code)]
        Err("arrêt de processus non pris en charge sur cette plateforme".into())
    }

    fn is_process_running(&self, process_id: u32) -> Result<bool, String> {
        let mut owned_processes = self
            .owned_processes
            .lock()
            .map_err(|_| "registre des processus possédés indisponible".to_string())?;
        let Some(child) = owned_processes.get_mut(&process_id) else {
            return Ok(false);
        };
        #[cfg(unix)]
        {
            let running = child
                .try_wait()
                .map_err(|error| format!("inspection du processus impossible: {error}"))?
                .is_none();
            if running {
                return Ok(true);
            }
            let _ = Command::new("kill")
                .args(["-TERM", &format!("-{process_id}")])
                .output();
            owned_processes.remove(&process_id);
            Ok(false)
        }

        #[cfg(windows)]
        {
            let _ = child;
            let running = self.windows_job_is_running(process_id)?;
            if !running {
                self.close_windows_job(process_id)?;
                owned_processes.remove(&process_id);
            }
            Ok(running)
        }
    }

    fn wait_service_tcp(
        &self,
        compose_file: &Path,
        service: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<(), String> {
        let started = Instant::now();
        let probe = format!("exec 3<>/dev/tcp/127.0.0.1/{port}");
        let server_root = compose_file
            .parent()
            .ok_or_else(|| "dossier serveur RealmBox introuvable".to_string())?;
        while started.elapsed() < timeout {
            let args = compose_args(compose_file, &["exec", "-T", service, "bash", "-c", &probe]);
            if self.run("docker", &args, Some(server_root)).is_ok() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(500));
        }
        Err(format!(
            "le service {service} n’est pas prêt sur son port {port} après {} s",
            timeout.as_secs()
        ))
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
    operation_sequence: u64,
    active_operation_id: String,
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
            operation_sequence: 0,
            active_operation_id: "launcher-0".into(),
        })
    }

    fn begin_operation(&mut self, name: &str) {
        self.operation_sequence = self.operation_sequence.saturating_add(1);
        self.active_operation_id = format!("{name}-{}", self.operation_sequence);
    }

    fn dialogue_chattiness(&self) -> DialogueChattiness {
        fs::read(self.app_data.join("dialogue-preferences.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn world_preferences(&self, record: Option<&InstallationRecord>) -> WorldPreferences {
        fs::read(self.app_data.join("world-preferences.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_else(|| WorldPreferences {
                bots_enabled: record.is_none_or(|record| record.bots_enabled),
                requested_bot_count: record.map_or_else(default_bot_count, |record| {
                    normalize_bot_count(record.bot_count)
                }),
                bot_presence: if record.is_some() {
                    legacy_bot_presence()
                } else {
                    BotPresence::default()
                },
            })
    }

    fn save_world_preferences(&self, preferences: WorldPreferences) -> Result<(), String> {
        write_atomic(
            &self.app_data.join("world-preferences.json"),
            &serde_json::to_vec_pretty(&preferences).map_err(|error| error.to_string())?,
        )
    }

    pub fn configure_dialogue_chattiness(
        &mut self,
        chattiness: DialogueChattiness,
    ) -> Result<LauncherStatus, String> {
        self.begin_operation("dialogue-chattiness");
        let record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        let running = self.client_process_id.is_some() || self.worldserver_is_running(&record)?;
        if running && (!record.ai_enabled || record.ai_model.is_none()) {
            return Err("les dialogues locaux ne sont pas actifs dans ce monde".into());
        }
        write_atomic(
            &self.app_data.join("dialogue-preferences.json"),
            &serde_json::to_vec_pretty(&chattiness).map_err(|error| error.to_string())?,
        )?;
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        write_ollama_chat_config(
            server_root,
            record.ai_enabled,
            record.ai_model.as_deref(),
            chattiness,
            dialogue_language_for_record(&record),
        )?;
        if running {
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
                        "printf 'reload config\\nollama reload\\n' > /proc/1/fd/0",
                    ],
                ),
                Some(server_root),
                &record.runtime_root.join("logs/dialogue-live-update.log"),
            )?;
        }
        Ok(self.installed_status(
            &record,
            if running {
                LauncherPhase::Running
            } else {
                LauncherPhase::Ready
            },
            if running {
                "Niveau de bavardage appliqué"
            } else {
                "Niveau de bavardage enregistré"
            },
            running,
        ))
    }

    pub fn inspect_ai_capability(&self) -> AiCapability {
        let mut capability = ai::inspect_ai_capability(&self.runner);
        if let Ok(available) = fs2::available_space(&self.app_data) {
            capability.disk_available_gb = Some(available as f32 / 1024_f32.powi(3));
            capability.disk_space_sufficient = capability
                .ollama_model
                .as_deref()
                .and_then(ai::model_download_bytes)
                .map(|required| available >= required + DISK_SAFETY_MARGIN_BYTES);
        }
        capability
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
                error_code: Some(ErrorCode::InstallationIncomplete),
                progress: 0,
                installed: false,
                recovery_available: find_latest_recovery_point(&self.app_data).is_ok(),
                bots_enabled: record.bots_enabled,
                bot_count: record.bot_count,
                requested_bot_count: self.world_preferences(Some(&record)).requested_bot_count,
                applied_bot_count: record.bot_count,
                bot_presence: self.world_preferences(Some(&record)).bot_presence,
                ai_enabled: record.ai_enabled,
                ai_model: record.ai_model.clone(),
                dialogue_chattiness: self.dialogue_chattiness(),
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
                error_code: Some(ErrorCode::from_detail(
                    &error,
                    ErrorCode::InstallationStateUnreadable,
                )),
                detail: Some(error),
                progress: 0,
                installed: false,
                recovery_available: find_latest_recovery_point(&self.app_data).is_ok(),
                bots_enabled: true,
                bot_count: default_bot_count(),
                requested_bot_count: default_bot_count(),
                applied_bot_count: default_bot_count(),
                bot_presence: BotPresence::default(),
                ai_enabled: false,
                ai_model: None,
                dialogue_chattiness: self.dialogue_chattiness(),
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

    pub fn inspect_installation(
        &self,
        model: Option<&str>,
    ) -> Result<crate::setup::InstallationCheck, String> {
        let extra = match model {
            Some(model) if ai::is_allowed_ollama_model(model) => ai::model_download_bytes(model)
                .ok_or_else(|| "taille du modèle inconnue".to_string())?,
            Some(_) => return Err("modèle local non autorisé".into()),
            None => 0,
        };
        Ok(crate::setup::inspect(
            &self.runner,
            self.ensure_fresh_install_target().is_ok(),
            platform_assets().is_ok(),
            fs2::available_space(&self.app_data).ok(),
            BASE_INSTALLATION_DISK_BYTES + extra,
        ))
    }

    pub fn bootstrap<F>(&mut self, progress: F) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        let _ = progress;
        let Some(mut record) = self.load_record()? else {
            return Ok(self.status());
        };
        self.resume_runtime_update_if_needed(&mut record)?;
        let running = self.worldserver_is_running(&record).unwrap_or(false);
        let docker_rebuild_required = !running
            && (self.app_data.join(DOCKER_RECOVERY_FILE).is_file()
                || (self
                    .runner
                    .run(
                        "docker",
                        &[
                            "info".into(),
                            "--format".into(),
                            "{{.ServerVersion}}".into(),
                        ],
                        None,
                    )
                    .is_ok()
                    && (!docker_volume_exists(&self.runner, DATABASE_VOLUME)
                        || !docker_volume_exists(&self.runner, SERVER_DATA_VOLUME))));
        Ok(self.installed_status(
            &record,
            if running {
                LauncherPhase::Running
            } else {
                LauncherPhase::Ready
            },
            if running {
                "Le monde est déjà lancé"
            } else if docker_rebuild_required {
                "Les ressources Docker seront reconstruites depuis la sauvegarde locale vérifiée au prochain lancement"
            } else {
                "Installation prête"
            },
            running,
        ))
    }

    pub fn change_game_data_path(
        &mut self,
        selected_path: &Path,
    ) -> Result<LauncherStatus, String> {
        self.begin_operation("change-game-data");
        let mut record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        if self.client_process_id.is_some() || self.worldserver_is_running(&record).unwrap_or(false)
        {
            return Err("arrêtez le monde avant de changer le dossier du client".into());
        }

        let game_data_root = validate_game_data_root(selected_path)?;
        if game_data_root == record.game_data_root {
            return Ok(self.installed_status(
                &record,
                LauncherPhase::Ready,
                "Dossier du client inchangé",
                false,
            ));
        }

        match record.client_choice {
            ClientChoice::ManagedOpenWow => {
                let managed_game = record.runtime_root.join("game");
                let staged_game = record.runtime_root.join(".game-path-update");
                let previous_game = record.runtime_root.join(".game-path-previous");

                if staged_game.exists() {
                    fs::remove_dir_all(&staged_game).map_err(|error| error.to_string())?;
                }
                if previous_game.exists() {
                    if managed_game.exists() {
                        fs::remove_dir_all(&previous_game).map_err(|error| error.to_string())?;
                    } else {
                        fs::rename(&previous_game, &managed_game)
                            .map_err(|error| error.to_string())?;
                    }
                }

                prepare_managed_openwow_game(
                    &self.runner,
                    &game_data_root,
                    &staged_game,
                    &self.addon_source,
                    &self.app_data.join("original-client-backup"),
                )?;
                fs::rename(&managed_game, &previous_game).map_err(|error| {
                    let _ = fs::remove_dir_all(&staged_game);
                    error.to_string()
                })?;
                if let Err(error) = fs::rename(&staged_game, &managed_game) {
                    let _ = fs::rename(&previous_game, &managed_game);
                    return Err(error.to_string());
                }

                let previous_root = record.game_data_root.clone();
                record.game_data_root = game_data_root;
                if let Err(error) = self.save_record(&record) {
                    record.game_data_root = previous_root;
                    let _ = fs::rename(&managed_game, &staged_game);
                    let _ = fs::rename(&previous_game, &managed_game);
                    let _ = fs::remove_dir_all(&staged_game);
                    return Err(error);
                }
                let _ = fs::remove_dir_all(&previous_game);
            }
            ClientChoice::OriginalWindows => {
                prepare_original_client_files(
                    &game_data_root,
                    &self.addon_source,
                    &self.app_data.join("original-client-backup"),
                )?;
                record.client_executable = game_data_root.join("Wow.exe");
                record.game_data_root = game_data_root;
                self.save_record(&record)?;
            }
        }

        Ok(self.installed_status(
            &record,
            LauncherPhase::Ready,
            "Dossier du client mis à jour",
            false,
        ))
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
        self.begin_operation("install");
        self.ensure_fresh_install_target()?;
        let InstallationOptions {
            client_choice,
            bots_enabled,
            bot_count,
            bot_presence,
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
        let required_disk_bytes = BASE_INSTALLATION_DISK_BYTES
            + ai_model
                .as_deref()
                .and_then(ai::model_download_bytes)
                .unwrap_or(0);
        ensure_available_space(&self.app_data, required_disk_bytes)?;
        self.emit_component(
            &mut progress,
            OperationComponent::GameData,
            OperationStep::Validate,
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
        let requested_bot_count = normalize_bot_count(bot_count);
        let bot_count =
            effective_playerbot_count(&docker_memory, bots_enabled, requested_bot_count);
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
                    self.download_component(
                        &mut progress,
                        OperationComponent::Client,
                        8,
                        20,
                        "Téléchargement du client OpenWoW",
                        platform.openwow_url,
                        platform.openwow_sha256,
                        &client_archive,
                        &logs.join("openwow-download.log"),
                    )?;
                    self.emit_component(
                        &mut progress,
                        OperationComponent::Client,
                        OperationStep::Verify,
                        LauncherPhase::Installing,
                        20,
                        "Vérification du client OpenWoW",
                        Some("Empreinte SHA-256 conforme"),
                    );
                    let client_root = staging.join("client");
                    self.emit_component(
                        &mut progress,
                        OperationComponent::Client,
                        OperationStep::Extract,
                        LauncherPhase::Installing,
                        21,
                        "Extraction du client OpenWoW",
                        None,
                    );
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
                self.emit_component(
                    &mut progress,
                    OperationComponent::Server,
                    OperationStep::Configure,
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
                install_realmbox_server_extensions(
                    &self.runner,
                    &self.addon_source,
                    &server_root,
                    ai_enabled,
                )?;
                write_realmbox_dockerfile(&server_root)?;
            }

            install_mmaps_config(&self.addon_source, &server_root)?;
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
                self.download_component(
                    &mut progress,
                    OperationComponent::Ai,
                    38,
                    46,
                    "Téléchargement du moteur de dialogue",
                    platform.ollama_url,
                    platform.ollama_sha256,
                    &archive,
                    &logs.join("ollama-download.log"),
                )?;
                self.emit_component(
                    &mut progress,
                    OperationComponent::Ai,
                    OperationStep::Verify,
                    LauncherPhase::Installing,
                    46,
                    "Vérification du moteur de dialogue",
                    Some("Empreinte SHA-256 conforme"),
                );
                let ai_root = staging.join("ai");
                fs::create_dir_all(&ai_root).map_err(|error| error.to_string())?;
                self.emit_component(
                    &mut progress,
                    OperationComponent::Ai,
                    OperationStep::Extract,
                    LauncherPhase::Installing,
                    47,
                    "Extraction du moteur de dialogue",
                    None,
                );
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

            self.emit_component(
                &mut progress,
                OperationComponent::Server,
                OperationStep::Download,
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

            self.emit_component(
                &mut progress,
                OperationComponent::Database,
                OperationStep::Configure,
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
            write_playerbots_config(&server_root, bots_enabled, bot_count, bot_presence)?;
            install_realmbox_presence_config(&self.addon_source, &server_root)?;
            write_realmbox_presence_config(&server_root, bots_enabled, bot_presence)?;
            self.emit_component(
                &mut progress,
                OperationComponent::Bots,
                OperationStep::Configure,
                LauncherPhase::Installing,
                84,
                "Configuration des compagnons",
                Some("Population adaptée à la mémoire Docker"),
            );
            write_ollama_chat_config(
                &server_root,
                ai_enabled,
                ai_model.as_deref(),
                self.dialogue_chattiness(),
                dialogue_language_for_game_data(&game_data_root),
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
                return Err(
                    "finalisation refusée : le runtime existant a été conservé pour protéger les données du royaume"
                        .into(),
                );
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
                runtime_release: Some(runtime_release_id()),
            };
            self.save_record(&record)?;
            self.save_world_preferences(WorldPreferences {
                bots_enabled,
                requested_bot_count,
                bot_presence,
            })?;
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
                    &compose_args(&compose_path, &["down", "--remove-orphans"]),
                    compose_path.parent(),
                    &staging.join("install-rollback.log"),
                );
            }
        }
        result
    }

    fn ensure_fresh_install_target(&self) -> Result<(), String> {
        let manifest = self.app_data.join("installation.json");
        let runtime = self.app_data.join(RUNTIME_DIRECTORY);
        for path in [&manifest, &runtime] {
            match fs::symlink_metadata(path) {
                Ok(_) => return Err(
                    "installation refusée : un royaume existe déjà. RealmBox ne réinstalle jamais par-dessus les personnages ; utilisez uniquement le parcours de mise à jour sécurisé".into(),
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("état d’installation incertain : {error}")),
            }
        }
        Ok(())
    }

    pub fn start<F>(
        &mut self,
        bots_enabled: Option<bool>,
        bot_count: Option<usize>,
        bot_presence: Option<BotPresence>,
        ai_enabled: Option<bool>,
        mut progress: F,
    ) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        self.begin_operation("start");
        let mut record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        if self.client_process_id.is_some() {
            return Err("le client RealmBox est déjà lancé".into());
        }
        self.resume_runtime_update_if_needed(&mut record)?;
        if self.worldserver_is_running(&record)? {
            return Err("le monde RealmBox est déjà lancé".into());
        }
        self.solo_profile_store(&record)?.resume_pending()?;
        let mut preferences = self.world_preferences(Some(&record));
        if let Some(enabled) = bots_enabled {
            preferences.bots_enabled = enabled;
        }
        if let Some(requested) = bot_count {
            preferences.requested_bot_count = normalize_bot_count(requested);
        }
        if let Some(presence) = bot_presence {
            preferences.bot_presence = presence;
        }
        record.bots_enabled = preferences.bots_enabled;
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
        self.save_world_preferences(preferences)?;
        self.save_record(&record)?;
        let mut server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?
            .to_path_buf();
        refresh_companion_addon(&self.addon_source, &record)?;
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
        let database_volume_missing = !docker_volume_exists(&self.runner, DATABASE_VOLUME);
        let server_data_volume_missing = !docker_volume_exists(&self.runner, SERVER_DATA_VOLUME);
        let database_recovery = prepare_docker_recovery(&self.app_data, database_volume_missing)?;
        let target_release = runtime_release_id();
        let requires_runtime_update =
            record.runtime_release.as_deref() != Some(target_release.as_str());
        let compose_repaired = if requires_runtime_update {
            false
        } else {
            repair_missing_local_server_image(&self.runner, &record)?
        };
        if database_recovery.is_some() || server_data_volume_missing || compose_repaired {
            self.emit(
                &mut progress,
                LauncherPhase::Recovering,
                6,
                "Reconstruction des ressources Docker",
                database_recovery
                    .as_ref()
                    .map(|_| "La dernière sauvegarde locale vérifiée sera restaurée"),
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
            Some(&server_root),
            &record.runtime_root.join("logs/start-database.log"),
        )?;

        if let Some(recovery) = &database_recovery {
            self.emit(
                &mut progress,
                LauncherPhase::Recovering,
                20,
                "Restauration des personnages",
                Some("Sauvegarde SQL locale vérifiée"),
            );
            restore_database_backup(
                &self.runner,
                &record.compose_file,
                &server_root,
                &recovery.backup,
                &record.runtime_root.join("logs/docker-recovery-import.log"),
            )?;
            validate_live_database_after_restore(
                &self.runner,
                &self.app_data,
                &record.compose_file,
                &server_root,
                &format!("docker-{}", recovery.stem),
                &record
                    .runtime_root
                    .join("logs/docker-recovery-validation.log"),
            )?;
        }

        let mut runtime_swapped_now = false;
        if requires_runtime_update {
            let images = embedded_server_images()?.ok_or_else(|| {
                "mise à jour refusée : cette build de développement ne contient pas les quatre images serveur immuables nécessaires au remplacement du runtime existant"
                    .to_string()
            })?;
            if record.runtime_release.as_deref()
                == Some(pending_runtime_release_id(&target_release).as_str())
            {
                validate_prepared_runtime_update(
                    &self.app_data,
                    &record,
                    &images,
                    &target_release,
                )?;
            } else {
                self.prepare_server_runtime_update(&mut record, &images, &mut progress)?;
                runtime_swapped_now = true;
                server_root = record
                    .compose_file
                    .parent()
                    .ok_or_else(|| "chemin serveur invalide".to_string())?
                    .to_path_buf();
            }
        }

        install_mmaps_config(&self.addon_source, &server_root)?;
        ensure_worldserver_console(&record.compose_file)?;
        ensure_mmaps_config_mount(&record.compose_file)?;
        ensure_restartable_server_data_extraction(&record.compose_file)?;
        write_ollama_chat_config(
            &server_root,
            record.ai_enabled,
            record.ai_model.as_deref(),
            self.dialogue_chattiness(),
            dialogue_language_for_record(&record),
        )?;
        record.bot_count = effective_playerbot_count(
            &docker_memory,
            record.bots_enabled,
            preferences.requested_bot_count,
        );
        self.save_record(&record)?;
        write_playerbots_config(
            &server_root,
            record.bots_enabled,
            record.bot_count,
            preferences.bot_presence,
        )?;
        install_realmbox_presence_config(&self.addon_source, &server_root)?;
        write_realmbox_presence_config(
            &server_root,
            record.bots_enabled,
            preferences.bot_presence,
        )?;

        if runtime_swapped_now {
            self.runner.run_long(
                "docker",
                &compose_args(
                    &record.compose_file,
                    &["up", "-d", "--wait", "--wait-timeout", "120", "database"],
                ),
                Some(&server_root),
                &record.runtime_root.join("logs/start-updated-database.log"),
            )?;
        }

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
            Some(&server_root),
            &record.runtime_root.join("logs/start-server-data.log"),
        )?;

        self.runner.run_long(
            "docker",
            &compose_args(&record.compose_file, &["run", "--rm", "db-import"]),
            Some(&server_root),
            &record.runtime_root.join("logs/start-db-import.log"),
        )?;
        if requires_runtime_update {
            record.runtime_release = Some(target_release);
            self.save_record(&record)?;
        }
        mark_local_realm_available(
            &self.runner,
            &record.compose_file,
            &server_root,
            &record.runtime_root.join("logs/start-realm.log"),
        )?;
        if database_recovery.is_some() {
            fs::remove_file(self.app_data.join(DOCKER_RECOVERY_FILE)).map_err(|error| {
                format!("marqueur de récupération Docker non supprimé: {error}")
            })?;
        }

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
                Some(&server_root),
                &record.runtime_root.join("logs/start-server.log"),
            )?;
            self.runner.wait_service_tcp(
                &record.compose_file,
                "authserver",
                3724,
                Duration::from_secs(180),
            )?;
            self.runner.wait_service_tcp(
                &record.compose_file,
                "worldserver",
                8085,
                Duration::from_secs(180),
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
                    Some(&server_root),
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

    pub fn configure_local_dialogue<F>(
        &mut self,
        enabled: bool,
        model: Option<String>,
        mut progress: F,
    ) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        self.begin_operation("dialogue");
        let mut record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        if self.client_process_id.is_some() || self.worldserver_is_running(&record)? {
            return Err(
                "arrêtez le monde avant de modifier les dialogues locaux afin que le serveur recharge leur configuration"
                    .into(),
            );
        }
        let mut server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?
            .to_path_buf();

        if !enabled {
            write_ollama_chat_config(
                &server_root,
                false,
                record.ai_model.as_deref(),
                self.dialogue_chattiness(),
                dialogue_language_for_record(&record),
            )?;
            if let Some(process_id) = self.ai_process_id.take() {
                self.runner.terminate(process_id)?;
            }
            record.ai_enabled = false;
            self.save_record(&record)?;
            self.emit(
                &mut progress,
                LauncherPhase::Ready,
                100,
                "Dialogues locaux désactivés",
                Some("Le modèle reste installé pour une réactivation sans téléchargement"),
            );
            return Ok(self.installed_status(
                &record,
                LauncherPhase::Ready,
                "Dialogues locaux désactivés",
                false,
            ));
        }

        if !record.bots_enabled {
            return Err("les dialogues locaux nécessitent les compagnons Playerbots".into());
        }
        let model = model.ok_or_else(|| {
            "RealmBox n’a reçu aucune recommandation exploitable de CanIRun".to_string()
        })?;
        if !ai::is_allowed_ollama_model(&model) {
            return Err("modèle Ollama refusé par la liste RealmBox".into());
        }
        let installed_executable = record
            .ollama_executable
            .as_ref()
            .filter(|path| path.is_file())
            .cloned();
        if local_dialogue_download_required(&record, &model) {
            ensure_available_space(
                &record.runtime_root,
                ai::model_download_bytes(&model).unwrap_or(0) + DISK_SAFETY_MARGIN_BYTES,
            )?;
        }
        let module_config_available = server_root
            .join("modules/mod-ollama-chat/conf/mod_ollama_chat.conf.dist")
            .is_file()
            || server_root
                .join("env/dist/etc/modules/mod_ollama_chat.conf")
                .is_file()
            || server_root
                .join("env/dist/etc/modules/mod_ollama_chat.conf.dist")
                .is_file();
        if !module_config_available {
            let images = embedded_server_images()?.ok_or_else(|| {
                "cette build RealmBox ne contient pas les images serveur immuables nécessaires à la mise à jour des dialogues"
                    .to_string()
            })?;
            self.prepare_server_runtime_update(&mut record, &images, &mut progress)?;
            server_root = record
                .compose_file
                .parent()
                .ok_or_else(|| "chemin serveur invalide".to_string())?
                .to_path_buf();
        }

        let platform = platform_assets()?;
        let logs = record.runtime_root.join("logs");
        fs::create_dir_all(&logs).map_err(|error| error.to_string())?;
        let executable = if let Some(executable) = installed_executable {
            if record.ai_model.as_deref() != Some(model.as_str()) {
                self.emit(
                    &mut progress,
                    LauncherPhase::Installing,
                    35,
                    "Téléchargement du modèle choisi par RealmBox",
                    Some(&model),
                );
                pull_ollama_model(
                    &self.runner,
                    &executable,
                    &record.runtime_root.join("ai/models"),
                    &model,
                    &logs,
                )?;
            }
            executable
        } else {
            let staging = record.runtime_root.join(".ai-installing");
            if staging.exists() {
                fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
            }
            let staged_ai = staging.join("ai");
            fs::create_dir_all(&staged_ai).map_err(|error| error.to_string())?;
            let archive = staging.join(platform.ollama_archive);
            self.emit(
                &mut progress,
                LauncherPhase::Installing,
                8,
                "Téléchargement du moteur de dialogue",
                Some("Ollama 0.33.2 · archive officielle vérifiée"),
            );
            self.download_component(
                &mut progress,
                OperationComponent::Ai,
                8,
                28,
                "Téléchargement du moteur de dialogue",
                platform.ollama_url,
                platform.ollama_sha256,
                &archive,
                &logs.join("ollama-download.log"),
            )?;
            extract_ollama(&self.runner, &archive, &staged_ai)?;
            let staged_executable = staged_ai.join(platform.ollama_executable);
            if !staged_executable.is_file() {
                return Err(
                    "l’archive Ollama vérifiée ne contient pas l’exécutable attendu".into(),
                );
            }
            verify_platform_ollama(&self.runner, &staged_ai)?;
            fs::remove_file(&archive).map_err(|error| error.to_string())?;
            self.emit(
                &mut progress,
                LauncherPhase::Installing,
                35,
                "Téléchargement du modèle choisi par RealmBox",
                Some(&model),
            );
            pull_ollama_model(
                &self.runner,
                &staged_executable,
                &staged_ai.join("models"),
                &model,
                &logs,
            )?;
            let ai_root = record.runtime_root.join("ai");
            if ai_root.exists() {
                fs::remove_dir_all(&ai_root).map_err(|error| error.to_string())?;
            }
            fs::rename(&staged_ai, &ai_root).map_err(|error| error.to_string())?;
            fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
            ai_root.join(platform.ollama_executable)
        };

        write_ollama_chat_config(
            &server_root,
            true,
            Some(&model),
            self.dialogue_chattiness(),
            dialogue_language_for_record(&record),
        )?;
        record.ai_enabled = true;
        record.ai_model = Some(model.clone());
        record.ollama_executable = Some(executable);
        record.ollama_sha256 = Some(platform.ollama_sha256.into());
        record.ollama_chat_commit = Some(OLLAMA_CHAT_COMMIT.into());
        self.save_record(&record)?;
        self.emit(
            &mut progress,
            LauncherPhase::Ready,
            100,
            "Dialogues locaux prêts",
            Some(&model),
        );
        Ok(self.installed_status(
            &record,
            LauncherPhase::Ready,
            "Dialogues locaux prêts",
            false,
        ))
    }

    fn compose_service_is_running(
        &self,
        record: &InstallationRecord,
        service: &str,
    ) -> Result<bool, String> {
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        let output = self.runner.run(
            "docker",
            &compose_args(
                &record.compose_file,
                &["ps", "--status", "running", "--services", service],
            ),
            Some(server_root),
        )?;
        Ok(output.lines().any(|line| line.trim() == service))
    }

    fn compose_service_is_running_bounded(
        &self,
        record: &InstallationRecord,
        service: &str,
        timeout: Duration,
    ) -> Result<bool, String> {
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        let output = self.runner.run_bounded(
            "docker",
            &compose_args(
                &record.compose_file,
                &["ps", "--status", "running", "--services", service],
            ),
            Some(server_root),
            timeout,
        )?;
        Ok(output.lines().any(|line| line.trim() == service))
    }

    fn worldserver_is_running(&self, record: &InstallationRecord) -> Result<bool, String> {
        self.compose_service_is_running(record, "worldserver")
    }

    pub fn inspect_realm_backup(&self) -> Result<Option<RealmBackupSummary>, String> {
        latest_realm_backup_summary(&self.app_data)
    }

    fn solo_profile_store(&self, record: &InstallationRecord) -> Result<SoloProfileStore, String> {
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        SoloProfileStore::new(
            &self.app_data,
            &server_root.join("env/dist/etc/worldserver.conf"),
            record.schema_version,
        )
    }

    pub fn inspect_solo_profiles(&self) -> Result<SoloProfileView, String> {
        let record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        self.solo_profile_store(&record)?.inspect()
    }

    fn ensure_solo_change_is_safe(&self, record: &InstallationRecord) -> Result<(), String> {
        if self.client_process_id.is_some() || self.worldserver_is_running(record)? {
            return Err("arrêtez le monde avant de modifier le profil solo".into());
        }
        if self.app_data.join(RUNTIME_UPDATE_FILE).exists() {
            return Err(
                "terminez la récupération du runtime avant de modifier le profil solo".into(),
            );
        }
        Ok(())
    }

    pub fn configure_solo_profile(
        &mut self,
        profile: SoloProfile,
    ) -> Result<SoloProfileView, String> {
        let record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        self.ensure_solo_change_is_safe(&record)?;
        let store = self.solo_profile_store(&record)?;
        store.resume_pending()?;
        store.apply(profile)
    }

    pub fn rollback_solo_profile(&mut self) -> Result<SoloProfileView, String> {
        let record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        self.ensure_solo_change_is_safe(&record)?;
        let store = self.solo_profile_store(&record)?;
        store.resume_pending()?;
        store.rollback()
    }

    pub fn query_local_guide(&self, query: LocalGuideQuery) -> Result<LocalGuideResponse, String> {
        let record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        if !docker_volume_exists_bounded(
            &self.runner,
            DATABASE_VOLUME,
            LOCAL_GUIDE_SHORT_COMMAND_TIMEOUT,
        ) {
            return Ok(LocalGuideResponse::unavailable());
        }
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        let database_was_running = self.compose_service_is_running_bounded(
            &record,
            "database",
            LOCAL_GUIDE_SHORT_COMMAND_TIMEOUT,
        )?;
        if !database_was_running {
            let start_result = self.runner.run_long_bounded(
                "docker",
                &compose_args(
                    &record.compose_file,
                    &[
                        "up",
                        "-d",
                        "--no-build",
                        "--pull",
                        "never",
                        "--no-deps",
                        "--wait",
                        "--wait-timeout",
                        "120",
                        "database",
                    ],
                ),
                Some(server_root),
                &record
                    .runtime_root
                    .join("logs/local-guide-database-start.log"),
                LOCAL_GUIDE_DATABASE_START_TIMEOUT,
            );
            if let Err(start_error) = start_result {
                // `up --wait` can fail after creating/starting the service. Only
                // stop it when this request observed it stopped beforehand.
                let stop_result = self.runner.run_long_bounded(
                    "docker",
                    &compose_args(&record.compose_file, &["stop", "database"]),
                    Some(server_root),
                    &record
                        .runtime_root
                        .join("logs/local-guide-database-stop.log"),
                    LOCAL_GUIDE_DATABASE_STOP_TIMEOUT,
                );
                return Err(match stop_result {
                    Ok(()) => start_error,
                    Err(stop_error) => format!(
                        "guide local indisponible : {start_error} ; arrêt de la base à vérifier : {stop_error}"
                    ),
                });
            }
        }

        let response = LocalGuideSearch::new(DockerLocalGuideSource {
            runner: &self.runner,
            compose_file: &record.compose_file,
            server_root,
            observed_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|duration| u64::try_from(duration.as_millis()).ok()),
        })
        .search(&query);

        if !database_was_running {
            self.runner.run_long_bounded(
                "docker",
                &compose_args(&record.compose_file, &["stop", "database"]),
                Some(server_root),
                &record
                    .runtime_root
                    .join("logs/local-guide-database-stop.log"),
                LOCAL_GUIDE_DATABASE_STOP_TIMEOUT,
            )?;
        }
        Ok(response)
    }

    pub fn create_realm_backup(&mut self) -> Result<RealmBackupSummary, String> {
        self.begin_operation("realm-backup");
        let record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas encore installé".to_string())?;
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        if !docker_volume_exists(&self.runner, DATABASE_VOLUME) {
            return Err(
                "le volume Docker des personnages a disparu ; lancez Jouer pour restaurer le dernier point vérifié avant de créer une sauvegarde"
                    .into(),
            );
        }
        let stem = next_manual_backup_stem(&self.app_data)?;
        let database_was_running = self.compose_service_is_running(&record, "database")?;

        if !database_was_running {
            self.runner.run_long(
                "docker",
                &compose_args(
                    &record.compose_file,
                    &["up", "-d", "--wait", "--wait-timeout", "120", "database"],
                ),
                Some(server_root),
                &record
                    .runtime_root
                    .join("logs/manual-backup-database-start.log"),
            )?;
        }

        let backup_result = create_database_backup(
            &self.runner,
            &self.app_data,
            &record.compose_file,
            server_root,
            &stem,
            &record.runtime_root.join("logs/manual-backup.log"),
        );
        let stop_result = if database_was_running {
            Ok(())
        } else {
            self.runner.run_long(
                "docker",
                &compose_args(&record.compose_file, &["stop", "database"]),
                Some(server_root),
                &record
                    .runtime_root
                    .join("logs/manual-backup-database-stop.log"),
            )
        };
        match (backup_result, stop_result) {
            (Ok(backup), Ok(())) => realm_backup_summary(&backup),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(stop_error)) => Err(format!(
                "sauvegarde vérifiée, mais la base temporairement démarrée n’a pas pu être arrêtée: {stop_error}"
            )),
            (Err(error), Err(stop_error)) => Err(format!(
                "{error}; la base temporairement démarrée n’a pas pu être arrêtée: {stop_error}"
            )),
        }
    }

    fn resume_runtime_update_if_needed(
        &mut self,
        record: &mut InstallationRecord,
    ) -> Result<bool, String> {
        let marker = self.app_data.join(RUNTIME_UPDATE_FILE);
        if !marker.is_file() {
            return Ok(false);
        }
        let transaction: RuntimeUpdateTransaction =
            serde_json::from_slice(&fs::read(&marker).map_err(|error| error.to_string())?)
                .map_err(|error| format!("marqueur de mise à jour serveur illisible: {error}"))?;
        let expected_transition = runtime_update_transition(
            transaction
                .recovery
                .source_runtime_release
                .as_deref()
                .unwrap_or("legacy"),
            &transaction.recovery.target_runtime_release,
            transaction.attempt,
        )?;
        if transaction.schema_version != 1
            || transaction.transition != transaction.recovery.stem
            || transaction.transition != expected_transition
            || transaction.recovery.schema_version != 1
            || transaction.recovery.target_runtime_release != runtime_release_id()
        {
            return Err(
                "marqueur de mise à jour serveur incohérent ; les runtimes sont conservés".into(),
            );
        }
        for image in [
            transaction.images.authserver.as_str(),
            transaction.images.worldserver.as_str(),
            transaction.images.db_import.as_str(),
            transaction.images.tools.as_str(),
        ] {
            validate_immutable_server_image(image)?;
        }

        let current_server = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?
            .to_path_buf();
        let staging_root = self
            .app_data
            .join(format!(".{}-staging", transaction.transition));
        let staged_server = staging_root.join("server");
        let rollback_root = self
            .app_data
            .join(RUNTIME_ROLLBACK_DIRECTORY)
            .join(&transaction.transition);
        let rollback_server = rollback_root.join("server");
        if rollback_root.exists() && !rollback_root.is_dir() {
            return Err("chemin de rollback occupé par un fichier inattendu".into());
        }
        fs::create_dir_all(&rollback_root).map_err(|error| error.to_string())?;
        let recovery_path = rollback_root.join("recovery.json");
        let expected_recovery =
            serde_json::to_vec_pretty(&transaction.recovery).map_err(|error| error.to_string())?;
        if recovery_path.exists() {
            let existing: RecoveryMetadata = serde_json::from_slice(
                &fs::read(&recovery_path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| format!("métadonnées de rollback illisibles: {error}"))?;
            if existing != transaction.recovery {
                return Err(
                    "métadonnées de rollback ambiguës ; RealmBox refuse de les écraser".into(),
                );
            }
        } else {
            write_atomic(&recovery_path, &expected_recovery)?;
        }

        let current_exists = current_server.join("compose.realmbox.yaml").is_file();
        let staged_exists = staged_server.join("compose.realmbox.yaml").is_file();
        let rollback_exists = rollback_server.join("compose.realmbox.yaml").is_file();
        if transaction.phase == RuntimeUpdatePhase::Published
            && (current_exists, staged_exists, rollback_exists) != (true, false, true)
        {
            return Err(
                "marqueur publié incohérent avec les runtimes présents ; aucun fichier n’est modifié"
                    .into(),
            );
        }
        match (current_exists, staged_exists, rollback_exists) {
            (true, true, false) => {
                if runtime_server_matches_images(&current_server, &transaction.images)?
                    || !runtime_server_matches_images(&staged_server, &transaction.images)?
                {
                    return Err(
                        "état de mise à jour ambigu avant publication ; aucun runtime n’est écrasé"
                            .into(),
                    );
                }
                fs::rename(&current_server, &rollback_server).map_err(|error| error.to_string())?;
                validate_recovery_point(&self.app_data, rollback_root.clone())?;
                fs::rename(&staged_server, &current_server)
                    .map_err(|error| format!("publication du runtime incomplète: {error}"))?;
            }
            (false, true, true) => {
                if !runtime_server_matches_images(&staged_server, &transaction.images)? {
                    return Err("le runtime stagé ne correspond pas aux images immuables".into());
                }
                validate_recovery_point(&self.app_data, rollback_root.clone())?;
                fs::rename(&staged_server, &current_server)
                    .map_err(|error| format!("reprise de publication impossible: {error}"))?;
            }
            (true, false, true) => {
                if !runtime_server_matches_images(&current_server, &transaction.images)? {
                    return Err(
                        "le runtime publié ne correspond pas au marqueur de mise à jour".into(),
                    );
                }
                validate_recovery_point(&self.app_data, rollback_root.clone())?;
            }
            _ => {
                return Err(
                    "combinaison de runtimes ambiguë ; RealmBox conserve chaque fichier et refuse de poursuivre"
                        .into(),
                );
            }
        }

        record.compose_file = current_server.join("compose.realmbox.yaml");
        record.ollama_chat_commit = Some(OLLAMA_CHAT_COMMIT.into());
        record.runtime_release = Some(pending_runtime_release_id(
            &transaction.recovery.target_runtime_release,
        ));
        self.save_record(record)?;
        fs::remove_file(&marker).map_err(|error| error.to_string())?;
        if staging_root.is_dir()
            && fs::read_dir(&staging_root)
                .map_err(|error| error.to_string())?
                .next()
                .is_none()
        {
            fs::remove_dir(&staging_root).map_err(|error| error.to_string())?;
        }
        Ok(true)
    }

    fn prepare_server_runtime_update<F>(
        &mut self,
        record: &mut InstallationRecord,
        images: &ServerImages,
        progress: &mut F,
    ) -> Result<(), String>
    where
        F: FnMut(LauncherProgress),
    {
        self.resume_runtime_update_if_needed(record)?;
        let target_release = runtime_release_id();
        if record.runtime_release.as_deref()
            == Some(pending_runtime_release_id(&target_release).as_str())
        {
            validate_prepared_runtime_update(&self.app_data, record, images, &target_release)?;
            return Ok(());
        }
        if record
            .runtime_release
            .as_deref()
            .is_some_and(|release| release.starts_with("pending:"))
        {
            return Err(
                "une autre mise à jour serveur incomplète doit être récupérée avant de poursuivre"
                    .into(),
            );
        }
        let current_server = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?
            .to_path_buf();
        let source_release = record
            .runtime_release
            .as_deref()
            .unwrap_or("legacy")
            .to_owned();
        let transition = migration_backup_stem(&source_release, &target_release);
        let recovery_metadata = RecoveryMetadata {
            schema_version: 1,
            stem: transition.clone(),
            source_runtime_release: record.runtime_release.clone(),
            target_runtime_release: target_release.clone(),
            ai_enabled: record.ai_enabled,
            ai_model: record.ai_model.clone(),
            ollama_chat_commit: record.ollama_chat_commit.clone(),
        };
        let staging_root = self.app_data.join(format!(".{transition}-staging"));
        let staged_server = staging_root.join("server");
        let rollback_root = self
            .app_data
            .join(RUNTIME_ROLLBACK_DIRECTORY)
            .join(&transition);

        if rollback_root.exists() {
            archive_consumed_runtime_rollback(&self.app_data, &rollback_root, &recovery_metadata)?;
        }
        if staging_root.exists() {
            fs::remove_dir_all(&staging_root).map_err(|error| error.to_string())?;
        }
        fs::create_dir_all(staged_server.join("env/dist/logs"))
            .map_err(|error| error.to_string())?;
        copy_file_preserving(&current_server.join(".env"), &staged_server.join(".env"))?;
        copy_directory(
            &current_server.join("env/dist/etc"),
            &staged_server.join("env/dist/etc"),
        )?;

        install_mmaps_config(&self.addon_source, &staged_server)?;
        let (uid, gid) = platform_container_ids(&self.runner)?;
        let compose = compose_file(
            uid.trim(),
            gid.trim(),
            &record.game_data_root,
            DEFAULT_DOCKER_BUILD_JOBS,
            Some(images),
        );
        let staged_compose = staged_server.join("compose.realmbox.yaml");
        write_atomic(&staged_compose, compose.as_bytes())?;

        self.emit(
            progress,
            LauncherPhase::Installing,
            6,
            "Préparation sécurisée du serveur",
            Some("Téléchargement du serveur immuable dans un staging séparé"),
        );
        self.runner.run_long(
            "docker",
            &compose_args(
                &staged_compose,
                &[
                    "pull",
                    "server-data-init",
                    "db-import",
                    "authserver",
                    "worldserver",
                ],
            ),
            Some(&staged_server),
            &record.runtime_root.join("logs/dialogue-runtime-pull.log"),
        )?;
        let module_dist = staged_server.join("env/dist/etc/modules/mod_ollama_chat.conf.dist");
        if let Some(parent) = module_dist.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        self.runner.run_to_file(
            "docker",
            &[
                "run".into(),
                "--rm".into(),
                "--entrypoint".into(),
                "cat".into(),
                images.worldserver.as_str().into(),
                "/azerothcore/env/ref/etc/modules/mod_ollama_chat.conf.dist".into(),
            ],
            None,
            &module_dist,
            &record.runtime_root.join("logs/dialogue-module-extract.log"),
        )?;
        let preferences = self.world_preferences(Some(record));
        write_playerbots_config(
            &staged_server,
            record.bots_enabled,
            record.bot_count,
            preferences.bot_presence,
        )?;
        install_realmbox_presence_config(&self.addon_source, &staged_server)?;
        write_realmbox_presence_config(
            &staged_server,
            record.bots_enabled,
            preferences.bot_presence,
        )?;
        write_ollama_chat_config(
            &staged_server,
            record.ai_enabled,
            record.ai_model.as_deref(),
            self.dialogue_chattiness(),
            dialogue_language_for_record(record),
        )?;

        self.emit(
            progress,
            LauncherPhase::Installing,
            18,
            "Sauvegarde des personnages",
            Some("Vérification complète obligatoire avant la mise à jour du serveur"),
        );
        self.runner.run_long(
            "docker",
            &compose_args(
                &record.compose_file,
                &["up", "-d", "--wait", "--wait-timeout", "120", "database"],
            ),
            Some(&current_server),
            &record
                .runtime_root
                .join("logs/dialogue-backup-database-start.log"),
        )?;
        let backup_result = create_pre_migration_backup(
            &self.runner,
            &self.app_data,
            &record.compose_file,
            &current_server,
            &source_release,
            &target_release,
            &record
                .runtime_root
                .join("logs/dialogue-pre-migration-backup.log"),
        );
        let stop_result = self.runner.run_long(
            "docker",
            &compose_args(&record.compose_file, &["down", "--remove-orphans"]),
            Some(&current_server),
            &record
                .runtime_root
                .join("logs/dialogue-backup-database-stop.log"),
        );
        backup_result.and(stop_result)?;

        self.emit(
            progress,
            LauncherPhase::Installing,
            28,
            "Mise à jour du serveur local",
            Some("L’ancien runtime reste disponible pour rollback"),
        );
        let transaction = RuntimeUpdateTransaction {
            schema_version: 1,
            transition,
            attempt: 1,
            phase: RuntimeUpdatePhase::Staged,
            images: images.clone(),
            recovery: recovery_metadata,
        };
        write_atomic(
            &self.app_data.join(RUNTIME_UPDATE_FILE),
            &serde_json::to_vec_pretty(&transaction).map_err(|error| error.to_string())?,
        )?;
        self.resume_runtime_update_if_needed(record)?;
        Ok(())
    }

    pub fn restore_last_recovery<F>(&mut self, mut progress: F) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        self.begin_operation("recovery");
        if self.client_process_id.is_some() || self.ai_process_id.is_some() {
            return Err("arrêtez le monde avant de restaurer son dernier état fonctionnel".into());
        }
        let mut original_record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas installé".to_string())?;
        self.resume_runtime_update_if_needed(&mut original_record)?;
        let point = find_latest_recovery_point(&self.app_data)?;
        let current_server = original_record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?
            .to_path_buf();
        let failed_root = self
            .app_data
            .join(RUNTIME_ROLLBACK_DIRECTORY)
            .join(format!("failed-after-{}", point.stem));
        let failed_server = failed_root.join("server");
        if failed_root.exists() {
            return Err(
                "un runtime postérieur est déjà conservé après une restauration ; RealmBox refuse de l’écraser"
                    .into(),
            );
        }

        self.emit(
            &mut progress,
            LauncherPhase::Recovering,
            10,
            "Vérification du point de restauration",
            Some("Runtime et sauvegarde SQL vérifiés avant toute modification"),
        );
        validate_recovery_point(&self.app_data, point.rollback_root.clone())?;
        let server_root = current_server.as_path();
        self.runner.run_long(
            "docker",
            &compose_args(
                &original_record.compose_file,
                &["up", "-d", "--wait", "--wait-timeout", "120", "database"],
            ),
            Some(server_root),
            &original_record
                .runtime_root
                .join("logs/restore-database-start.log"),
        )?;
        let safety_backup = create_pre_migration_backup(
            &self.runner,
            &self.app_data,
            &original_record.compose_file,
            server_root,
            original_record
                .runtime_release
                .as_deref()
                .unwrap_or("unknown-current"),
            &format!("restore-safety-{}", point.stem),
            &original_record
                .runtime_root
                .join("logs/restore-safety-backup.log"),
        )?;

        self.emit(
            &mut progress,
            LauncherPhase::Recovering,
            32,
            "Conservation de l’état actuel",
            Some("Une sauvegarde de sécurité non écrasable a été créée"),
        );
        self.runner.run_long(
            "docker",
            &compose_args(&original_record.compose_file, &["down", "--remove-orphans"]),
            Some(server_root),
            &original_record
                .runtime_root
                .join("logs/restore-current-stop.log"),
        )?;
        fs::create_dir_all(&failed_root).map_err(|error| error.to_string())?;
        fs::rename(&current_server, &failed_server).map_err(|error| error.to_string())?;
        if let Err(error) = fs::rename(&point.rollback_server, &current_server) {
            let _ = fs::rename(&failed_server, &current_server);
            return Err(format!("restauration du runtime annulée : {error}"));
        }

        let mut restored_record = original_record.clone();
        restored_record.compose_file = current_server.join("compose.realmbox.yaml");
        restored_record.runtime_release = point
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.source_runtime_release.clone());
        restored_record.ai_enabled = point
            .metadata
            .as_ref()
            .is_some_and(|metadata| metadata.ai_enabled);
        restored_record.ai_model = point
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.ai_model.clone());
        restored_record.ollama_chat_commit = point
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.ollama_chat_commit.clone());

        let restoration = (|| {
            self.emit(
                &mut progress,
                LauncherPhase::Recovering,
                55,
                "Restauration des personnages",
                Some("Import du dernier dump complet vérifié"),
            );
            self.runner.run_long(
                "docker",
                &compose_args(
                    &restored_record.compose_file,
                    &["up", "-d", "--wait", "--wait-timeout", "120", "database"],
                ),
                Some(&current_server),
                &restored_record
                    .runtime_root
                    .join("logs/restore-target-database-start.log"),
            )?;
            restore_database_backup(
                &self.runner,
                &restored_record.compose_file,
                &current_server,
                &point.backup,
                &restored_record.runtime_root.join("logs/restore-import.log"),
            )?;
            validate_live_database_after_restore(
                &self.runner,
                &self.app_data,
                &restored_record.compose_file,
                &current_server,
                &point.stem,
                &restored_record
                    .runtime_root
                    .join("logs/restore-validation.log"),
            )?;
            self.runner.run_long(
                "docker",
                &compose_args(&restored_record.compose_file, &["down", "--remove-orphans"]),
                Some(&current_server),
                &restored_record
                    .runtime_root
                    .join("logs/restore-target-stop.log"),
            )?;
            self.save_record(&restored_record)
        })();

        if let Err(error) = restoration {
            let _ = self.runner.run_long(
                "docker",
                &compose_args(&restored_record.compose_file, &["down", "--remove-orphans"]),
                Some(&current_server),
                &original_record
                    .runtime_root
                    .join("logs/restore-failed-stop.log"),
            );
            let runtime_reverted = fs::rename(&current_server, &point.rollback_server)
                .and_then(|_| fs::rename(&failed_server, &current_server));
            let database_reverted = self
                .runner
                .run_long(
                    "docker",
                    &compose_args(
                        &original_record.compose_file,
                        &["up", "-d", "--wait", "--wait-timeout", "120", "database"],
                    ),
                    Some(&current_server),
                    &original_record
                        .runtime_root
                        .join("logs/restore-revert-database-start.log"),
                )
                .and_then(|_| {
                    restore_database_backup(
                        &self.runner,
                        &original_record.compose_file,
                        &current_server,
                        &safety_backup,
                        &original_record
                            .runtime_root
                            .join("logs/restore-revert-import.log"),
                    )
                })
                .and_then(|_| {
                    self.runner.run_long(
                        "docker",
                        &compose_args(&original_record.compose_file, &["down", "--remove-orphans"]),
                        Some(&current_server),
                        &original_record
                            .runtime_root
                            .join("logs/restore-revert-stop.log"),
                    )
                });
            return match (runtime_reverted, database_reverted) {
                (Ok(()), Ok(())) => {
                    let _ = fs::remove_dir(&failed_root);
                    Err(format!(
                        "restauration annulée et état précédent rétabli : {error}"
                    ))
                }
                (runtime, database) => Err(format!(
                    "restauration interrompue ; récupération automatique incomplète ({error}) ; runtime={runtime:?}, base={database:?}"
                )),
            };
        }

        archive_recovery_directory(&point.rollback_root, &point.stem)?;

        self.emit(
            &mut progress,
            LauncherPhase::Ready,
            100,
            "Dernier état fonctionnel restauré",
            Some("Le monde reste arrêté jusqu’à l’action Jouer"),
        );
        Ok(self.installed_status(
            &restored_record,
            LauncherPhase::Ready,
            "Dernier état fonctionnel restauré",
            false,
        ))
    }

    pub fn stop<F>(&mut self, mut progress: F) -> Result<LauncherStatus, String>
    where
        F: FnMut(LauncherProgress),
    {
        self.begin_operation("stop");
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
        bot_presence: BotPresence,
    ) -> Result<LauncherStatus, String> {
        self.begin_operation("bot-experience");
        let mut record = self
            .load_record()?
            .ok_or_else(|| "RealmBox n’est pas installé".to_string())?;
        let server_root = record
            .compose_file
            .parent()
            .ok_or_else(|| "chemin serveur invalide".to_string())?;
        let container_id = self
            .runner
            .run(
                "docker",
                &compose_args(&record.compose_file, &["ps", "-q", "worldserver"]),
                Some(server_root),
            )
            .map_err(|error| format!("état du monde Docker indisponible: {error}"))?;
        let container_id = container_id.trim();
        let running = !container_id.is_empty();
        if !running && self.client_process_id.is_some() {
            return Err(
                "état incohérent : le client est ouvert mais le worldserver Docker est absent"
                    .into(),
            );
        }
        if running {
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
        }
        let docker_memory = self
            .runner
            .run(
                "docker",
                &["info".into(), "--format".into(), "{{.MemTotal}}".into()],
                None,
            )
            .unwrap_or_default();
        let effective_count = if docker_memory.trim().parse::<u64>().is_ok() {
            effective_playerbot_count(&docker_memory, bots_enabled, requested_count)
        } else if bots_enabled {
            record.bot_count.max(5)
        } else {
            0
        };
        write_playerbots_config(server_root, bots_enabled, effective_count, bot_presence)?;
        install_realmbox_presence_config(&self.addon_source, server_root)?;
        write_realmbox_presence_config(server_root, bots_enabled, bot_presence)?;
        let disable_dialogue = !bots_enabled && record.ai_enabled;
        if disable_dialogue {
            write_ollama_chat_config(
                server_root,
                false,
                record.ai_model.as_deref(),
                self.dialogue_chattiness(),
                dialogue_language_for_record(&record),
            )?;
        }
        if running {
            let commands = if disable_dialogue {
                "printf 'reload config\\nplayerbots rndbot reload\\nplayerbots rndbot update\\nollama reload\\n' > /proc/1/fd/0"
            } else {
                "printf 'reload config\\nplayerbots rndbot reload\\nplayerbots rndbot update\\n' > /proc/1/fd/0"
            };
            self.runner.run_long(
                "docker",
                &compose_args(
                    &record.compose_file,
                    &["exec", "-T", "worldserver", "sh", "-lc", commands],
                ),
                Some(server_root),
                &record.runtime_root.join("logs/playerbots-live-update.log"),
            )?;
        }
        record.bots_enabled = bots_enabled;
        record.bot_count = effective_count;
        if !bots_enabled {
            record.ai_enabled = false;
            if let Some(process_id) = self.ai_process_id.take() {
                self.runner.terminate(process_id)?;
            }
        }
        self.save_record(&record)?;
        self.save_world_preferences(WorldPreferences {
            bots_enabled,
            requested_bot_count: normalize_bot_count(requested_count),
            bot_presence,
        })?;
        Ok(self.installed_status(
            &record,
            if running {
                LauncherPhase::Running
            } else {
                LauncherPhase::Ready
            },
            if running {
                "Population et présence mises à jour"
            } else {
                "Population et présence enregistrées"
            },
            running,
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
        self.emit_component(
            progress,
            OperationComponent::Launcher,
            match phase {
                LauncherPhase::Installing => OperationStep::Configure,
                LauncherPhase::Starting => OperationStep::Start,
                LauncherPhase::Stopping => OperationStep::Stop,
                LauncherPhase::Recovering => OperationStep::Restore,
                LauncherPhase::Ready | LauncherPhase::Running => OperationStep::Complete,
                LauncherPhase::NeedsGameData | LauncherPhase::Error => OperationStep::Validate,
            },
            phase,
            value,
            message,
            detail,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_component<F>(
        &self,
        progress: &mut F,
        component: OperationComponent,
        step: OperationStep,
        phase: LauncherPhase,
        value: u8,
        message: &str,
        detail: Option<&str>,
    ) where
        F: FnMut(LauncherProgress),
    {
        progress(LauncherProgress {
            operation_id: self.active_operation_id.clone(),
            component,
            step,
            phase,
            message: message.into(),
            detail: detail.map(str::to_owned),
            error_code: None,
            progress: value,
            completed_bytes: None,
            total_bytes: None,
            cancellable: false,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn download_component<F>(
        &self,
        progress: &mut F,
        component: OperationComponent,
        start_percent: u8,
        end_percent: u8,
        message: &str,
        url: &str,
        expected_sha256: &str,
        destination: &Path,
        log_path: &Path,
    ) -> Result<(), String>
    where
        F: FnMut(LauncherProgress),
    {
        self.runner.download_verified(
            url,
            expected_sha256,
            destination,
            log_path,
            &mut |completed_bytes, total_bytes| {
                let range = end_percent.saturating_sub(start_percent) as u64;
                let progress_percent = total_bytes
                    .filter(|total| *total > 0)
                    .map(|total| {
                        start_percent
                            .saturating_add(((completed_bytes.min(total) * range) / total) as u8)
                    })
                    .unwrap_or(start_percent);
                progress(LauncherProgress {
                    operation_id: self.active_operation_id.clone(),
                    component,
                    step: OperationStep::Download,
                    phase: LauncherPhase::Installing,
                    message: message.into(),
                    detail: None,
                    error_code: None,
                    progress: progress_percent,
                    completed_bytes: Some(completed_bytes),
                    total_bytes,
                    cancellable: false,
                });
            },
        )
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
            return Err(format!(
                "le manifeste du royaume utilise le schéma {:?}, différent du schéma {INSTALL_SCHEMA} pris en charge ; les données sont conservées et aucune réinstallation automatique n’est autorisée",
                schema_version
            ));
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
            error_code: None,
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
            recovery_available: find_latest_recovery_point(&self.app_data).is_ok(),
            bots_enabled: record.bots_enabled,
            bot_count: record.bot_count,
            requested_bot_count: self.world_preferences(Some(record)).requested_bot_count,
            applied_bot_count: record.bot_count,
            bot_presence: self.world_preferences(Some(record)).bot_presence,
            ai_enabled: record.ai_enabled,
            ai_model: record.ai_model.clone(),
            dialogue_chattiness: self.dialogue_chattiness(),
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
        error_code: None,
        progress: 0,
        installed: false,
        recovery_available: false,
        bots_enabled: true,
        bot_count: default_bot_count(),
        requested_bot_count: default_bot_count(),
        applied_bot_count: default_bot_count(),
        bot_presence: BotPresence::default(),
        ai_enabled: false,
        ai_model: None,
        dialogue_chattiness: DialogueChattiness::default(),
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

fn download_verified_http(
    url: &str,
    expected_sha256: &str,
    destination: &Path,
    log_path: &Path,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), String> {
    use reqwest::{
        StatusCode,
        blocking::Client,
        header::{CONTENT_RANGE, RANGE},
    };

    if destination.is_file() && verify_sha256(destination, expected_sha256).is_ok() {
        let total = fs::metadata(destination)
            .map_err(|error| error.to_string())?
            .len();
        progress(total, Some(total));
        return Ok(());
    }
    if destination.exists() {
        fs::remove_file(destination).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut partial_name = destination
        .file_name()
        .ok_or_else(|| "nom de téléchargement invalide".to_string())?
        .to_os_string();
    partial_name.push(".part");
    let partial = destination.with_file_name(partial_name);
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(30 * 60))
        .build()
        .map_err(|error| format!("initialisation du téléchargement impossible: {error}"))?;
    let mut last_error = "téléchargement interrompu".to_string();

    for attempt in 1..=3 {
        let existing = fs::metadata(&partial)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let mut request = client.get(url);
        if existing > 0 {
            request = request.header(RANGE, format!("bytes={existing}-"));
        }
        let mut response = match request.send() {
            Ok(response) => response,
            Err(error) => {
                last_error = format!("tentative {attempt}/3: {error}");
                let _ = writeln!(
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(log_path)
                        .map_err(|error| error.to_string())?,
                    "{last_error}"
                );
                continue;
            }
        };
        if response.status() == StatusCode::RANGE_NOT_SATISFIABLE {
            if existing > 0 && verify_sha256(&partial, expected_sha256).is_ok() {
                fs::rename(&partial, destination).map_err(|error| error.to_string())?;
                progress(existing, Some(existing));
                return Ok(());
            }
            let _ = fs::remove_file(&partial);
            last_error = format!("tentative {attempt}/3: reprise HTTP refusée");
            continue;
        }
        if !response.status().is_success() {
            last_error = format!("tentative {attempt}/3: serveur HTTP {}", response.status());
            continue;
        }

        let resumed = response.status() == StatusCode::PARTIAL_CONTENT && existing > 0;
        let starting_offset = if resumed { existing } else { 0 };
        let total = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.rsplit('/').next())
            .and_then(|value| value.parse::<u64>().ok())
            .or_else(|| {
                response
                    .content_length()
                    .map(|remaining| starting_offset + remaining)
            });
        let downloaded = match append_download_body(
            &mut response,
            &partial,
            resumed,
            starting_offset,
            total,
            progress,
        ) {
            Ok(downloaded) => downloaded,
            Err(error) => {
                last_error = format!("tentative {attempt}/3: {error}");
                continue;
            }
        };
        if total.is_some_and(|total| downloaded != total) {
            last_error = format!(
                "tentative {attempt}/3: fichier incomplet ({downloaded}/{} octets)",
                total.unwrap_or_default()
            );
            continue;
        }
        publish_verified_download(&partial, destination, expected_sha256)?;
        let _ = writeln!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .map_err(|error| error.to_string())?,
            "téléchargement vérifié: {downloaded} octets"
        );
        return Ok(());
    }
    let _ = fs::remove_file(&partial);
    Err(format!(
        "téléchargement abandonné après trois tentatives: {last_error}"
    ))
}

fn append_download_body(
    response: &mut dyn Read,
    partial: &Path,
    resumed: bool,
    starting_offset: u64,
    total: Option<u64>,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64, String> {
    let mut output = OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(partial)
        .map_err(|error| error.to_string())?;
    let mut downloaded = starting_offset;
    progress(downloaded, total);
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = response.read(&mut buffer).map_err(|error| {
            let _ = output.flush();
            error.to_string()
        })?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
        downloaded += read as u64;
        progress(downloaded, total);
    }
    output.flush().map_err(|error| error.to_string())?;
    output.sync_all().map_err(|error| error.to_string())?;
    Ok(downloaded)
}

fn publish_verified_download(
    partial: &Path,
    destination: &Path,
    expected_sha256: &str,
) -> Result<(), String> {
    if let Err(error) = verify_sha256(partial, expected_sha256) {
        let _ = fs::remove_file(partial);
        return Err(error);
    }
    fs::rename(partial, destination).map_err(|error| error.to_string())
}

fn ensure_available_space(path: &Path, required_bytes: u64) -> Result<(), String> {
    let available = fs2::available_space(path).map_err(|error| {
        format!(
            "impossible de mesurer l’espace disponible pour {}: {error}",
            path.display()
        )
    })?;
    if available < required_bytes {
        return Err(format!(
            "espace disque insuffisant : {} Gio requis avec marge de sécurité, {} Gio disponibles",
            required_bytes.div_ceil(1024_u64.pow(3)),
            available / 1024_u64.pow(3)
        ));
    }
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let actual = sha256_file(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "empreinte SHA-256 invalide pour {}: attendue {expected}, reçue {actual}",
            path.display()
        ))
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
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
    Ok(format!("{:x}", digest.finalize()))
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

fn install_realmbox_server_extensions<R: CommandRunner>(
    runner: &R,
    addon_source: &Path,
    server_root: &Path,
    ai_enabled: bool,
) -> Result<(), String> {
    let resource_root = realmbox_resource_root(addon_source)?;
    let presence_source = resource_root
        .join("server-modules")
        .join(REALMBOX_PRESENCE_MODULE);
    let presence_destination = server_root.join("modules").join(REALMBOX_PRESENCE_MODULE);
    copy_directory(&presence_source, &presence_destination)?;

    if !ai_enabled {
        return Ok(());
    }

    let patch = resource_root.join("patches").join(REALMBOX_OLLAMA_PATCH);
    let patch_contents = fs::read_to_string(&patch).map_err(|error| {
        format!(
            "correctif des dialogues RealmBox absent ({}): {error}",
            patch.display()
        )
    })?;
    let expected_metadata = format!("Upstream-Commit: {OLLAMA_CHAT_COMMIT}");
    if !patch_contents.lines().any(|line| line == expected_metadata) {
        return Err("le correctif des dialogues ne correspond pas au commit épinglé".into());
    }

    let ollama_root = server_root.join("modules/mod-ollama-chat");
    for action in [["apply", "--check"], ["apply", ""]] {
        let mut args = vec![
            "-C".into(),
            ollama_root.as_os_str().into(),
            action[0].into(),
        ];
        if !action[1].is_empty() {
            args.push(action[1].into());
        }
        args.push(patch.as_os_str().into());
        runner.run("git", &args, None)?;
    }
    Ok(())
}

fn realmbox_resource_root(addon_source: &Path) -> Result<PathBuf, String> {
    let bundled = addon_source.parent().and_then(Path::parent);
    if let Some(root) = bundled
        && root
            .join("server-modules")
            .join(REALMBOX_PRESENCE_MODULE)
            .is_dir()
    {
        return Ok(root.to_path_buf());
    }

    let development = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    if development
        .join("server-modules")
        .join(REALMBOX_PRESENCE_MODULE)
        .is_dir()
    {
        return Ok(development);
    }

    Err("racine des ressources RealmBox introuvable".into())
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
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };

        let source = temporary
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let destination = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let moved = unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }

    #[cfg(not(windows))]
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

fn copy_file_preserving(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_file() {
        return Err(format!(
            "fichier requis absent pendant la mise à jour : {}",
            source.display()
        ));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::copy(source, destination)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "dossier requis absent pendant la mise à jour : {}",
            source.display()
        ));
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            copy_file_preserving(&entry.path(), &target)?;
        } else {
            return Err(format!(
                "lien ou fichier spécial refusé dans la configuration runtime : {}",
                entry.path().display()
            ));
        }
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
    let backup = backup_root
        .join(source_id(game_data_root))
        .join(format!("managed-openwow-realmlist-{locale}.wtf"));
    if realmlist.is_file() && !backup.exists() {
        fs::create_dir_all(backup.parent().expect("backup parent"))
            .map_err(|error| error.to_string())?;
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
        let backup = backup_root
            .join(source_id(game_data_root))
            .join(format!("realmlist-{locale}.wtf"));
        fs::create_dir_all(backup.parent().expect("backup parent"))
            .map_err(|error| error.to_string())?;
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
        let contents = fs::read(&source).map_err(|error| error.to_string())?;
        write_atomic(&addon_destination.join(filename), &contents)?;
    }
    Ok(())
}

fn refresh_companion_addon(addon_source: &Path, record: &InstallationRecord) -> Result<(), String> {
    let game_root = match record.client_choice {
        ClientChoice::ManagedOpenWow => record.runtime_root.join("game"),
        ClientChoice::OriginalWindows => record.game_data_root.clone(),
    };
    install_companion_addon(addon_source, &game_root)
}

fn install_realmbox_presence_config(addon_source: &Path, server_root: &Path) -> Result<(), String> {
    let resource_root = realmbox_resource_root(addon_source)?;
    copy_file_preserving(
        &resource_root
            .join("server-modules")
            .join(REALMBOX_PRESENCE_MODULE)
            .join("conf/realmbox-presence.conf.dist"),
        &server_root.join("env/dist/etc/modules/realmbox-presence.conf.dist"),
    )
}

fn install_mmaps_config(addon_source: &Path, server_root: &Path) -> Result<(), String> {
    let resource_root = realmbox_resource_root(addon_source)?;
    copy_file_preserving(
        &resource_root.join("runtime/mmaps-config.yaml"),
        &server_root.join("mmaps-config.yaml"),
    )
}

#[derive(Debug, Clone, Copy)]
struct BotPresenceTuning {
    relocation_enabled: bool,
    scan_interval_ms: &'static str,
    target_fraction: &'static str,
    minimum_per_player: &'static str,
    maximum_per_player: &'static str,
    nearby_radius: &'static str,
    spawn_min_radius: &'static str,
    spawn_max_radius: &'static str,
    bot_cooldown_seconds: &'static str,
    autonomy_return_seconds: &'static str,
    active_alone_percent: &'static str,
    active_radius: &'static str,
    force_active_in_zone: &'static str,
    real_player_weight: &'static str,
    minimum_native_teleport: &'static str,
    maximum_native_teleport: &'static str,
}

fn bot_presence_tuning(profile: BotPresence) -> BotPresenceTuning {
    match profile {
        BotPresence::Dispersed => BotPresenceTuning {
            relocation_enabled: false,
            scan_interval_ms: "5000",
            target_fraction: "0.0",
            minimum_per_player: "0",
            maximum_per_player: "0",
            nearby_radius: "300.0",
            spawn_min_radius: "160.0",
            spawn_max_radius: "280.0",
            bot_cooldown_seconds: "600",
            autonomy_return_seconds: "60",
            active_alone_percent: "5",
            active_radius: "325",
            force_active_in_zone: "0",
            real_player_weight: "1.0",
            minimum_native_teleport: "3600",
            maximum_native_teleport: "18000",
        },
        BotPresence::Natural => BotPresenceTuning {
            relocation_enabled: true,
            scan_interval_ms: "2000",
            target_fraction: "0.30",
            minimum_per_player: "3",
            maximum_per_player: "15",
            nearby_radius: "220.0",
            spawn_min_radius: "90.0",
            spawn_max_radius: "180.0",
            bot_cooldown_seconds: "300",
            autonomy_return_seconds: "600",
            active_alone_percent: "10",
            active_radius: "250",
            force_active_in_zone: "0",
            real_player_weight: "5.0",
            minimum_native_teleport: "1800",
            maximum_native_teleport: "7200",
        },
        BotPresence::Close => BotPresenceTuning {
            relocation_enabled: true,
            scan_interval_ms: "1000",
            target_fraction: "0.60",
            minimum_per_player: "4",
            maximum_per_player: "30",
            nearby_radius: "150.0",
            spawn_min_radius: "50.0",
            spawn_max_radius: "110.0",
            bot_cooldown_seconds: "60",
            autonomy_return_seconds: "900",
            active_alone_percent: "10",
            active_radius: "300",
            force_active_in_zone: "1",
            real_player_weight: "15.0",
            minimum_native_teleport: "900",
            maximum_native_teleport: "3600",
        },
    }
}

fn write_realmbox_presence_config(
    server_root: &Path,
    bots_enabled: bool,
    profile: BotPresence,
) -> Result<(), String> {
    let tuning = bot_presence_tuning(profile);
    ensure_module_config_key_from_dist(
        server_root,
        "realmbox-presence.conf",
        "RealmBoxPresence.AutonomyReturnSeconds",
    )?;
    write_module_config(
        server_root,
        REALMBOX_PRESENCE_MODULE,
        "realmbox-presence.conf",
        &[
            (
                "RealmBoxPresence.Enabled",
                u8::from(bots_enabled && tuning.relocation_enabled).to_string(),
            ),
            (
                "RealmBoxPresence.ScanIntervalMs",
                tuning.scan_interval_ms.into(),
            ),
            ("RealmBoxPresence.PlayerCooldownSeconds", "0".into()),
            (
                "RealmBoxPresence.TargetFraction",
                tuning.target_fraction.into(),
            ),
            (
                "RealmBoxPresence.MinBotsPerPlayer",
                tuning.minimum_per_player.into(),
            ),
            (
                "RealmBoxPresence.MaxBotsPerPlayer",
                tuning.maximum_per_player.into(),
            ),
            ("RealmBoxPresence.NearbyRadius", tuning.nearby_radius.into()),
            (
                "RealmBoxPresence.SpawnMinRadius",
                tuning.spawn_min_radius.into(),
            ),
            (
                "RealmBoxPresence.SpawnMaxRadius",
                tuning.spawn_max_radius.into(),
            ),
            ("RealmBoxPresence.MaxMovesPerScan", "1".into()),
            (
                "RealmBoxPresence.BotCooldownSeconds",
                tuning.bot_cooldown_seconds.into(),
            ),
            (
                "RealmBoxPresence.AutonomyReturnSeconds",
                tuning.autonomy_return_seconds.into(),
            ),
            ("RealmBoxPresence.ReleasedBotGraceSeconds", "300".into()),
        ],
    )
}

fn ensure_module_config_key_from_dist(
    server_root: &Path,
    filename: &str,
    key: &str,
) -> Result<(), String> {
    let destination = server_root.join("env/dist/etc/modules").join(filename);
    if !destination.is_file() {
        return Ok(());
    }

    let prefix = format!("{key} =");
    let mut config = fs::read_to_string(&destination).map_err(|error| error.to_string())?;
    let existing = config
        .lines()
        .filter(|line| line.starts_with(&prefix))
        .count();
    if existing == 1 {
        return Ok(());
    }
    if existing > 1 {
        return Err(format!(
            "la configuration gérée contient plusieurs clés {key} ; mise à jour refusée"
        ));
    }

    let distributed = destination.with_file_name(format!("{filename}.dist"));
    let distributed_config = fs::read_to_string(&distributed).map_err(|error| error.to_string())?;
    let mut candidates = distributed_config
        .lines()
        .filter(|line| line.starts_with(&prefix));
    let line = candidates.next().ok_or_else(|| {
        format!("la nouvelle configuration épinglée ne contient pas la clé {key}")
    })?;
    if candidates.next().is_some() {
        return Err(format!(
            "la nouvelle configuration épinglée contient plusieurs clés {key}"
        ));
    }

    if !config.ends_with('\n') {
        config.push('\n');
    }
    config.push_str(line);
    config.push('\n');
    write_atomic(&destination, config.as_bytes())
}

fn write_playerbots_config(
    server_root: &Path,
    enabled: bool,
    requested_count: usize,
    presence: BotPresence,
) -> Result<(), String> {
    let value = if enabled { 1 } else { 0 };
    let count = if enabled { requested_count.max(1) } else { 0 };
    let tuning = bot_presence_tuning(presence);
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
            ("AiPlayerbot.DisabledWithoutRealPlayer", value.to_string()),
            (
                "AiPlayerbot.DisabledWithoutRealPlayerLoginDelay",
                "5".to_string(),
            ),
            (
                "AiPlayerbot.DisabledWithoutRealPlayerLogoutDelay",
                "60".to_string(),
            ),
            (
                "AiPlayerbot.BotActiveAlone",
                tuning.active_alone_percent.to_string(),
            ),
            (
                "AiPlayerbot.BotActiveAloneForceWhenInRadius",
                tuning.active_radius.to_string(),
            ),
            (
                "AiPlayerbot.BotActiveAloneForceWhenInZone",
                tuning.force_active_in_zone.to_string(),
            ),
            ("AiPlayerbot.BotActiveAloneForceWhenInMap", "0".to_string()),
            ("AiPlayerbot.LevelBrackets.Enabled", value.to_string()),
            ("AiPlayerbot.LevelBrackets.CheckFrequency", "60".to_string()),
            (
                "AiPlayerbot.LevelBrackets.Dynamic.UseDynamicDistribution",
                value.to_string(),
            ),
            (
                "AiPlayerbot.LevelBrackets.Dynamic.RealPlayerWeight",
                tuning.real_player_weight.to_string(),
            ),
            (
                "AiPlayerbot.LevelBrackets.Dynamic.SyncFactions",
                value.to_string(),
            ),
            ("AiPlayerbot.AutoTeleportForLevel", value.to_string()),
            ("AiPlayerbot.RandomBotTeleLowerLevel", "1".to_string()),
            ("AiPlayerbot.RandomBotTeleHigherLevel", "3".to_string()),
            (
                "AiPlayerbot.MinRandomBotTeleportInterval",
                tuning.minimum_native_teleport.to_string(),
            ),
            (
                "AiPlayerbot.MaxRandomBotTeleportInterval",
                tuning.maximum_native_teleport.to_string(),
            ),
            ("AiPlayerbot.ProbTeleToBankers", "0.25".to_string()),
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

fn ensure_mmaps_config_mount(compose_path: &Path) -> Result<(), String> {
    const MOUNT: &str =
        "      - ./mmaps-config.yaml:/azerothcore/env/dist/bin/mmaps-config.yaml:ro\n";
    let source = fs::read_to_string(compose_path).map_err(|error| error.to_string())?;
    if source.contains(MOUNT) {
        return Ok(());
    }
    const ANCHOR: &str = "      - realmbox-server-data:/work\n";
    if source.matches(ANCHOR).count() != 1 {
        return Err("configuration server-data-init non reconnue".into());
    }
    let updated = source.replacen(ANCHOR, &format!("{ANCHOR}{MOUNT}"), 1);
    write_atomic(compose_path, updated.as_bytes())
}

fn ensure_restartable_server_data_extraction(compose_path: &Path) -> Result<(), String> {
    const LEGACY_CLEANUPS: [&str; 2] = [
        "          rm -rf /work/vmaps /work/mmaps;\n",
        "          rm -rf /work/Buildings /work/vmaps /work/mmaps;\n",
    ];
    const CLEANUPS: [(&str, &str); 3] = [
        (
            "          rm -rf /work/Buildings;\n",
            "          /azerothcore/env/dist/bin/vmap4_extractor;\n",
        ),
        (
            "          rm -rf /work/vmaps;\n",
            "          mkdir -p /work/vmaps;\n",
        ),
        (
            "          rm -rf /work/mmaps;\n",
            "          /azerothcore/env/dist/bin/mmaps_generator --config /azerothcore/env/dist/bin/mmaps-config.yaml --silent;\n",
        ),
    ];
    let source = fs::read_to_string(compose_path).map_err(|error| error.to_string())?;
    let mut updated = source.clone();
    for legacy in LEGACY_CLEANUPS {
        updated = updated.replace(legacy, "");
    }
    for (cleanup, anchor) in CLEANUPS {
        if updated.contains(cleanup) {
            continue;
        }
        if updated.matches(anchor).count() != 1 {
            return Err("commande d’extraction des données serveur non reconnue".into());
        }
        updated = updated.replacen(anchor, &format!("{cleanup}{anchor}"), 1);
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

pub(crate) fn playerbot_capacity(memory_output: &str) -> usize {
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
    line.to_lowercase()
        .split(|character: char| !character.is_alphabetic())
        .any(|word| {
            matches!(
                word,
                "error"
                    | "failed"
                    | "failure"
                    | "fatal"
                    | "panic"
                    | "warning"
                    | "warn"
                    | "erreur"
                    | "échec"
            )
        })
}

fn redact_diagnostic_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.to_ascii_lowercase().contains("password")
        || trimmed.to_ascii_lowercase().contains("authorization")
        || trimmed.to_ascii_lowercase().contains("token=")
    {
        "[ligne sensible masquée]".into()
    } else {
        let mut redacted = trimmed.to_owned();
        for variable in ["HOME", "USERPROFILE"] {
            if let Some(home) =
                std::env::var_os(variable).and_then(|value| value.into_string().ok())
                && !home.is_empty()
            {
                redacted = redacted.replace(&home, "[HOME]");
            }
        }
        redact_user_directory(&redacted).chars().take(500).collect()
    }
}

fn redact_user_directory(line: &str) -> String {
    for prefix in ["/Users/", "C:\\Users\\"] {
        if let Some(start) = line.find(prefix) {
            let user_start = start + prefix.len();
            let separator = if prefix.starts_with('/') { '/' } else { '\\' };
            if let Some(end) = line[user_start..].find(separator) {
                let mut redacted = line.to_owned();
                redacted.replace_range(user_start..user_start + end, "[USER]");
                return redacted;
            }
        }
    }
    line.to_owned()
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
    } else if combined.contains("worldserver")
        || combined.contains("authserver")
        || combined.contains("server-data")
    {
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

fn game_data_has_locale(game_data_root: &Path, locale: &str) -> bool {
    find_directory_case_insensitive(&game_data_root.join("Data"), locale).is_some()
}

fn dialogue_language_for_game_data(game_data_root: &Path) -> DialogueLanguage {
    if game_data_has_locale(game_data_root, "frFR") {
        DialogueLanguage::French
    } else {
        DialogueLanguage::English
    }
}

fn dialogue_language_for_record(record: &InstallationRecord) -> DialogueLanguage {
    if game_data_has_locale(&record.game_data_root, "frFR")
        || game_data_has_locale(&record.runtime_root.join("game"), "frFR")
    {
        DialogueLanguage::French
    } else {
        DialogueLanguage::English
    }
}

fn local_dialogue_download_required(record: &InstallationRecord, model: &str) -> bool {
    record.ai_model.as_deref() != Some(model)
        || record
            .ollama_executable
            .as_ref()
            .is_none_or(|executable| !executable.is_file())
}

fn dialogue_prompts(
    language: DialogueLanguage,
) -> (&'static str, &'static str, &'static str, &'static str) {
    match language {
        DialogueLanguage::French => (
            OLLAMA_DIALOGUE_SYSTEM_PROMPT_FR,
            OLLAMA_RANDOM_PROMPT_FR,
            OLLAMA_RANDOM_VARIATIONS_FR,
            OLLAMA_EVENT_PROMPT_FR,
        ),
        DialogueLanguage::English => (
            OLLAMA_DIALOGUE_SYSTEM_PROMPT_EN,
            OLLAMA_RANDOM_PROMPT_EN,
            OLLAMA_RANDOM_VARIATIONS_EN,
            OLLAMA_EVENT_PROMPT_EN,
        ),
    }
}

fn write_ollama_chat_config(
    server_root: &Path,
    enabled: bool,
    model: Option<&str>,
    chattiness: DialogueChattiness,
    language: DialogueLanguage,
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
    let chatter = match chattiness {
        DialogueChattiness::Quiet => (
            "0", "0", "0", "0", "100", "0", "0", "60", "0", "2", "4", "180", "360", "1", "100",
        ),
        DialogueChattiness::Balanced => (
            "1", "20", "8", "1", "100", "20", "50", "90", "0", "2", "4", "90", "180", "2", "100",
        ),
        DialogueChattiness::Lively => (
            "1", "35", "10", "2", "100", "35", "100", "60", "0", "4", "6", "30", "90", "2", "100",
        ),
    };
    let (
        automatic_chatter,
        random_chance,
        event_chance,
        event_self_chance,
        player_reply_chance,
        bot_say_reply_chance,
        bot_party_reply_chance,
        per_bot_cooldown,
        per_scope_cooldown,
        scope_rate_limit,
        global_rate_limit,
        min_random_interval,
        max_random_interval,
        max_chain_depth,
        chain_chance_decay,
    ) = chatter;
    let (system_prompt, random_prompt, random_variations, event_prompt) =
        dialogue_prompts(language);
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
            ("OllamaChat.CapabilityProbeTimeoutSeconds", "5".into()),
            ("OllamaChat.HttpTimeoutSeconds", "120".into()),
            ("OllamaChat.Model", model.to_owned()),
            ("OllamaChat.NumPredict", "96".into()),
            ("OllamaChat.ReasoningTokenReserve", "256".into()),
            ("OllamaChat.Temperature", "0".into()),
            ("OllamaChat.TopP", "0.9".into()),
            ("OllamaChat.NumCtx", "3072".into()),
            ("OllamaChat.SystemPrompt", system_prompt.into()),
            (
                "OllamaChat.ChatPromptTemplate",
                OLLAMA_DIALOGUE_CHAT_PROMPT.into(),
            ),
            ("OllamaChat.MaxConcurrentQueries", "1".into()),
            ("OllamaChat.WorkerThreads", "1".into()),
            ("OllamaChat.MaxQueueDepth", "4".into()),
            ("OllamaChat.ThinkMode", r#""off""#.into()),
            ("OllamaChat.ThinkModeEnableForModule", "0".into()),
            ("OllamaChat.DebugEnabled", "0".into()),
            ("OllamaChat.DebugShowFullPrompt", "0".into()),
            ("OllamaChat.EnableChatHistory", "0".into()),
            ("OllamaChat.ConversationHistorySaveInterval", "0".into()),
            ("OllamaChat.EnableChatBotSnapshotTemplate", "0".into()),
            ("OllamaChat.Memory.Enable", "0".into()),
            ("OllamaChat.Relationship.Enable", "0".into()),
            ("OllamaChat.EnableRAG", "0".into()),
            ("OllamaChat.EnableEmoteReactions", "0".into()),
            (
                "OllamaChat.PlayerReplyChance.Say",
                player_reply_chance.into(),
            ),
            (
                "OllamaChat.PlayerReplyChance.Channel",
                player_reply_chance.into(),
            ),
            (
                "OllamaChat.PlayerReplyChance.Party",
                player_reply_chance.into(),
            ),
            (
                "OllamaChat.PlayerReplyChance.Guild",
                player_reply_chance.into(),
            ),
            ("OllamaChat.BotReplyChance.Say", bot_say_reply_chance.into()),
            ("OllamaChat.BotReplyChance.Channel", "0".into()),
            (
                "OllamaChat.BotReplyChance.Party",
                bot_party_reply_chance.into(),
            ),
            ("OllamaChat.BotReplyChance.Guild", "0".into()),
            ("OllamaChat.EnableWhisperReplies", enabled.clone()),
            ("OllamaChat.MaxBotsToPick", "1".into()),
            (
                "OllamaChat.BotConversation.MaxChainDepth",
                max_chain_depth.into(),
            ),
            (
                "OllamaChat.BotConversation.ChanceDecayPct",
                chain_chance_decay.into(),
            ),
            ("OllamaChat.BotConversation.RequireRecentHuman", "0".into()),
            ("OllamaChat.Cooldown.PerBotSeconds", per_bot_cooldown.into()),
            (
                "OllamaChat.Cooldown.PerScopeSeconds",
                per_scope_cooldown.into(),
            ),
            (
                "OllamaChat.RateLimit.ScopePerMinute",
                scope_rate_limit.into(),
            ),
            (
                "OllamaChat.RateLimit.GlobalPerMinute",
                global_rate_limit.into(),
            ),
            ("OllamaChat.EnableRandomChatter", automatic_chatter.into()),
            ("OllamaChat.MinRandomInterval", min_random_interval.into()),
            ("OllamaChat.MaxRandomInterval", max_random_interval.into()),
            ("OllamaChat.RandomChatterRealPlayerDistance", "150.0".into()),
            (
                "OllamaChat.RandomChatterPromptTemplate",
                random_prompt.into(),
            ),
            (
                "OllamaChat.RandomChatterPromptVariations",
                random_variations.into(),
            ),
            (
                "OllamaChat.RandomChatterBotCommentChance",
                random_chance.into(),
            ),
            ("OllamaChat.RandomChatterMaxBotsPerPlayer", "1".into()),
            ("OllamaChat.EnableEventChatter", automatic_chatter.into()),
            (
                "OllamaChat.EventChatterBotCommentChance",
                event_chance.into(),
            ),
            (
                "OllamaChat.EventChatterBotSelfCommentChance",
                event_self_chance.into(),
            ),
            ("OllamaChat.EventChatterMaxBotsPerPlayer", "1".into()),
            ("OllamaChat.EventChatterPromptTemplate", event_prompt.into()),
            ("OllamaChat.DisableRepliesInCombat", "1".into()),
            ("OllamaChat.DisableForCustomChannels", "1".into()),
            ("OllamaChat.DisableForGuild", "1".into()),
            ("OllamaChat.DisableForParty", "0".into()),
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
    result
        .and_then(|()| verify_ollama_model_manifest(models, model))
        .and(stop_result)
}

fn verify_ollama_model_manifest(models: &Path, model: &str) -> Result<(), String> {
    let expected = ai::expected_ollama_digest(model)
        .ok_or_else(|| "modèle Ollama refusé par la liste RealmBox".to_string())?;
    let (name, tag) = model
        .split_once(':')
        .ok_or_else(|| "tag Ollama incomplet".to_string())?;
    let manifest = models
        .join("manifests/registry.ollama.ai/library")
        .join(name)
        .join(tag);
    verify_sha256(&manifest, expected).map_err(|error| {
        format!("le manifeste du modèle téléchargé ne correspond pas au modèle épinglé: {error}")
    })
}

fn runtime_release_id() -> String {
    format!("{}-schema-{INSTALL_SCHEMA}", env!("CARGO_PKG_VERSION"))
}

fn pending_runtime_release_id(target_release: &str) -> String {
    format!("pending:{target_release}")
}

fn migration_backup_stem(source_release: &str, target_release: &str) -> String {
    let digest = Sha256::digest(format!("{source_release}\0{target_release}"));
    format!("pre-migration-{}", &format!("{digest:x}")[..16])
}

fn runtime_update_transition(
    source_release: &str,
    target_release: &str,
    attempt: u32,
) -> Result<String, String> {
    if attempt == 0 {
        return Err("numéro de tentative de mise à jour invalide".into());
    }
    let base = migration_backup_stem(source_release, target_release);
    Ok(if attempt == 1 {
        base
    } else {
        format!("{base}-attempt-{attempt}")
    })
}

fn validate_database_backup(path: &Path) -> Result<(), String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() == 0 {
        return Err("la sauvegarde SQL est vide ; migration annulée".into());
    }
    let mut found = [false; 4];
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|error| error.to_string())?;
        for (index, database) in [
            "acore_auth",
            "acore_characters",
            "acore_playerbots",
            "acore_world",
        ]
        .iter()
        .enumerate()
        {
            found[index] |= line.contains(database);
        }
        if found.iter().all(|present| *present) {
            return Ok(());
        }
    }
    Err("la sauvegarde SQL ne contient pas toutes les bases RealmBox ; migration annulée".into())
}

fn validate_database_backup_pair(backup: PathBuf) -> Result<DatabaseBackup, String> {
    let stem = backup
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|stem| stem.starts_with("pre-migration-") || stem.starts_with("manual-backup-"))
        .ok_or_else(|| "nom de sauvegarde RealmBox invalide".to_string())?
        .to_owned();
    let checksum = backup.with_extension("sha256");
    if !checksum.is_file() {
        return Err(format!("somme SHA-256 absente pour {stem}"));
    }
    let expected = fs::read_to_string(&checksum).map_err(|error| error.to_string())?;
    verify_sha256(&backup, expected.trim())?;
    validate_database_backup(&backup)?;
    Ok(DatabaseBackup { stem, backup })
}

fn find_latest_database_backup(app_data: &Path) -> Result<DatabaseBackup, String> {
    let root = app_data.join(PLAYER_DATA_BACKUP_DIRECTORY);
    if !root.is_dir() {
        return Err("aucune sauvegarde locale des personnages n’est disponible".into());
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.path().extension() != Some(OsStr::new("sql")) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok();
        candidates.push((modified, entry.path()));
    }
    candidates.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, candidate) in candidates {
        if let Ok(backup) = validate_database_backup_pair(candidate) {
            return Ok(backup);
        }
    }
    Err("aucune sauvegarde complète et vérifiée des personnages n’est disponible".into())
}

fn realm_backup_summary(backup: &Path) -> Result<RealmBackupSummary, String> {
    let metadata = fs::metadata(backup).map_err(|error| error.to_string())?;
    let created_at_unix_ms = metadata
        .modified()
        .map_err(|error| error.to_string())?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "date de sauvegarde RealmBox invalide".to_string())?
        .as_millis()
        .try_into()
        .map_err(|_| "date de sauvegarde RealmBox hors limites".to_string())?;
    Ok(RealmBackupSummary {
        created_at_unix_ms,
        size_bytes: metadata.len(),
    })
}

fn latest_realm_backup_summary(app_data: &Path) -> Result<Option<RealmBackupSummary>, String> {
    let root = app_data.join(PLAYER_DATA_BACKUP_DIRECTORY);
    if !root.is_dir() {
        return Ok(None);
    }
    let mut has_backup = false;
    for entry in fs::read_dir(&root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.path().extension() == Some(OsStr::new("sql")) {
            has_backup = true;
            break;
        }
    }
    if !has_backup {
        return Ok(None);
    }
    let backup = find_latest_database_backup(app_data)?;
    realm_backup_summary(&backup.backup).map(Some)
}

fn next_manual_backup_stem(app_data: &Path) -> Result<String, String> {
    let backup_root = app_data.join(PLAYER_DATA_BACKUP_DIRECTORY);
    fs::create_dir_all(&backup_root).map_err(|error| error.to_string())?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "horloge système invalide pour la sauvegarde".to_string())?
        .as_millis();
    let base = format!("manual-backup-{timestamp}");
    for attempt in 1_u32.. {
        let stem = if attempt == 1 {
            base.clone()
        } else {
            format!("{base}-{attempt}")
        };
        if !["sql", "sha256", "partial"]
            .iter()
            .any(|extension| backup_root.join(&stem).with_extension(extension).exists())
        {
            return Ok(stem);
        }
    }
    unreachable!("u32 manual backup namespace exhausted")
}

fn prepare_docker_recovery(
    app_data: &Path,
    database_volume_missing: bool,
) -> Result<Option<DatabaseBackup>, String> {
    let marker = app_data.join(DOCKER_RECOVERY_FILE);
    if marker.is_file() {
        let record: DockerRecoveryRecord =
            serde_json::from_slice(&fs::read(&marker).map_err(|error| error.to_string())?)
                .map_err(|error| format!("marqueur de récupération Docker illisible: {error}"))?;
        if record.schema_version != 1 {
            return Err(
                "version inconnue du marqueur de récupération Docker ; les données sont conservées"
                    .into(),
            );
        }
        let backup = app_data
            .join(PLAYER_DATA_BACKUP_DIRECTORY)
            .join(format!("{}.sql", record.backup_stem));
        let backup = validate_database_backup_pair(backup)?;
        if backup.stem != record.backup_stem {
            return Err("marqueur de récupération Docker incohérent".into());
        }
        return Ok(Some(backup));
    }
    if !database_volume_missing {
        return Ok(None);
    }
    let backup = find_latest_database_backup(app_data).map_err(|error| {
        format!(
            "le volume Docker des personnages a disparu et {error} ; reconstruction arrêtée pour ne pas créer un royaume vide"
        )
    })?;
    let record = DockerRecoveryRecord {
        schema_version: 1,
        backup_stem: backup.stem.clone(),
    };
    write_atomic(
        &marker,
        &serde_json::to_vec_pretty(&record).map_err(|error| error.to_string())?,
    )?;
    Ok(Some(backup))
}

fn create_pre_migration_backup<R: CommandRunner>(
    runner: &R,
    app_data: &Path,
    compose_file: &Path,
    server_root: &Path,
    source_release: &str,
    target_release: &str,
    error_log: &Path,
) -> Result<PathBuf, String> {
    create_database_backup(
        runner,
        app_data,
        compose_file,
        server_root,
        &migration_backup_stem(source_release, target_release),
        error_log,
    )
}

fn create_database_backup<R: CommandRunner>(
    runner: &R,
    app_data: &Path,
    compose_file: &Path,
    server_root: &Path,
    stem: &str,
    error_log: &Path,
) -> Result<PathBuf, String> {
    if !(stem.starts_with("pre-migration-") || stem.starts_with("manual-backup-"))
        || !stem
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("nom de sauvegarde RealmBox invalide".into());
    }
    let backup_root = app_data.join(PLAYER_DATA_BACKUP_DIRECTORY);
    fs::create_dir_all(&backup_root).map_err(|error| error.to_string())?;
    let backup = backup_root.join(format!("{stem}.sql"));
    let checksum = backup_root.join(format!("{stem}.sha256"));

    if backup.exists() || checksum.exists() {
        if !backup.is_file() || !checksum.is_file() {
            return Err(
                "une sauvegarde RealmBox incomplète existe déjà ; elle est conservée et l’opération est annulée"
                    .into(),
            );
        }
        let expected = fs::read_to_string(&checksum).map_err(|error| error.to_string())?;
        verify_sha256(&backup, expected.trim())?;
        validate_database_backup(&backup)?;
        return Ok(backup);
    }

    let temporary = backup_root.join(format!("{stem}.partial"));
    write_secret_atomic(&temporary, b"")?;
    let dump_command = r#"exec mysqldump --user=root --password="$MYSQL_ROOT_PASSWORD" --all-databases --single-transaction --quick --routines --events --triggers --hex-blob --no-tablespaces --set-gtid-purged=OFF"#;
    runner.run_to_file(
        "docker",
        &compose_args(
            compose_file,
            &["exec", "-T", "database", "sh", "-c", dump_command],
        ),
        Some(server_root),
        &temporary,
        error_log,
    )?;
    OpenOptions::new()
        .write(true)
        .open(&temporary)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())?;
    validate_database_backup(&temporary)?;
    let digest = sha256_file(&temporary)?;
    fs::rename(&temporary, &backup).map_err(|error| error.to_string())?;
    write_atomic(&checksum, format!("{digest}\n").as_bytes())?;
    Ok(backup)
}

fn validate_recovery_point(
    app_data: &Path,
    rollback_root: PathBuf,
) -> Result<RecoveryPoint, String> {
    let stem = rollback_root
        .file_name()
        .and_then(OsStr::to_str)
        .filter(|name| name.starts_with("pre-migration-"))
        .ok_or_else(|| "nom de rollback RealmBox invalide".to_string())?
        .to_owned();
    let rollback_server = rollback_root.join("server");
    if !rollback_server.join("compose.realmbox.yaml").is_file() {
        return Err(format!("runtime de rollback incomplet: {stem}"));
    }
    let backup_root = app_data.join(PLAYER_DATA_BACKUP_DIRECTORY);
    let backup = backup_root.join(format!("{stem}.sql"));
    let checksum = backup_root.join(format!("{stem}.sha256"));
    if !backup.is_file() || !checksum.is_file() {
        return Err(format!("sauvegarde SQL du rollback absente: {stem}"));
    }
    let expected = fs::read_to_string(&checksum).map_err(|error| error.to_string())?;
    verify_sha256(&backup, expected.trim())?;
    validate_database_backup(&backup)?;

    let metadata_path = rollback_root.join("recovery.json");
    let metadata = if metadata_path.is_file() {
        let bytes = fs::read(&metadata_path).map_err(|error| error.to_string())?;
        let metadata: RecoveryMetadata =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if metadata.schema_version != 1 || metadata.stem != stem {
            return Err(format!("métadonnées de rollback incohérentes: {stem}"));
        }
        Some(metadata)
    } else {
        None
    };

    Ok(RecoveryPoint {
        stem,
        backup,
        rollback_root,
        rollback_server,
        metadata,
    })
}

fn find_latest_recovery_point(app_data: &Path) -> Result<RecoveryPoint, String> {
    let root = app_data.join(RUNTIME_ROLLBACK_DIRECTORY);
    if !root.is_dir() {
        return Err("aucun état fonctionnel restaurable n’est disponible".into());
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        if let Ok(point) = validate_recovery_point(app_data, entry.path()) {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok();
            candidates.push((modified, point));
        }
    }
    candidates.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    candidates
        .into_iter()
        .next()
        .map(|(_, point)| point)
        .ok_or_else(|| "aucun état fonctionnel complet et vérifié n’est disponible".into())
}

fn archive_recovery_directory(rollback_root: &Path, stem: &str) -> Result<PathBuf, String> {
    let parent = rollback_root
        .parent()
        .ok_or_else(|| "dossier parent du rollback introuvable".to_string())?;
    let base = format!("restored-{stem}");
    for attempt in 1_u32.. {
        let name = if attempt == 1 {
            base.clone()
        } else {
            format!("{base}-{attempt}")
        };
        let destination = parent.join(name);
        if destination.exists() {
            continue;
        }
        fs::rename(rollback_root, &destination).map_err(|error| error.to_string())?;
        return Ok(destination);
    }
    unreachable!("u32 recovery archive namespace exhausted")
}

fn archive_consumed_runtime_rollback(
    app_data: &Path,
    rollback_root: &Path,
    expected: &RecoveryMetadata,
) -> Result<PathBuf, String> {
    if rollback_root.join("server").exists() {
        return Err(
            "un rollback complet de mise à jour existe déjà ; RealmBox refuse de l’écraser".into(),
        );
    }
    let metadata: RecoveryMetadata = serde_json::from_slice(
        &fs::read(rollback_root.join("recovery.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("métadonnées du rollback consommé illisibles: {error}"))?;
    if &metadata != expected {
        return Err(
            "rollback consommé ambigu ; RealmBox le conserve et refuse de le réutiliser".into(),
        );
    }
    validate_database_backup_pair(
        app_data
            .join(PLAYER_DATA_BACKUP_DIRECTORY)
            .join(format!("{}.sql", expected.stem)),
    )?;
    archive_recovery_directory(rollback_root, &expected.stem)
}

fn validate_prepared_runtime_update(
    app_data: &Path,
    record: &InstallationRecord,
    images: &ServerImages,
    target_release: &str,
) -> Result<(), String> {
    if record.runtime_release.as_deref()
        != Some(pending_runtime_release_id(target_release).as_str())
    {
        return Err("état de mise à jour serveur inattendu".into());
    }
    let compose = fs::read_to_string(&record.compose_file).map_err(|error| error.to_string())?;
    for (service, expected) in [
        ("server-data-init", images.tools.as_str()),
        ("db-import", images.db_import.as_str()),
        ("authserver", images.authserver.as_str()),
        ("worldserver", images.worldserver.as_str()),
    ] {
        if compose_service_image(&compose, service).as_deref() != Some(expected) {
            return Err(format!(
                "runtime préparé invalide : l’image immuable de {service} ne correspond pas à la release"
            ));
        }
    }

    let rollback_root = app_data.join(RUNTIME_ROLLBACK_DIRECTORY);
    let entries = fs::read_dir(&rollback_root).map_err(|_| {
        "runtime préparé invalide : aucun rollback vérifiable n’est disponible".to_string()
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let Ok(point) = validate_recovery_point(app_data, entry.path()) else {
            continue;
        };
        if point.metadata.as_ref().is_some_and(|metadata| {
            metadata.target_runtime_release == target_release && metadata.stem == point.stem
        }) {
            return Ok(());
        }
    }
    Err(
        "runtime préparé invalide : sauvegarde complète et rollback de cette release introuvables"
            .into(),
    )
}

fn restore_database_backup<R: CommandRunner>(
    runner: &R,
    compose_file: &Path,
    server_root: &Path,
    backup: &Path,
    log_path: &Path,
) -> Result<(), String> {
    validate_database_backup(backup)?;
    let command = r#"exec mysql --user=root --password="$MYSQL_ROOT_PASSWORD""#;
    runner.run_with_input(
        "docker",
        &compose_args(
            compose_file,
            &["exec", "-T", "database", "sh", "-c", command],
        ),
        Some(server_root),
        backup,
        log_path,
    )
}

fn validate_live_database_after_restore<R: CommandRunner>(
    runner: &R,
    app_data: &Path,
    compose_file: &Path,
    server_root: &Path,
    stem: &str,
    error_log: &Path,
) -> Result<(), String> {
    let validation = app_data.join(format!(".restore-validation-{stem}.partial"));
    if validation.exists() {
        return Err(
            "une validation de restauration précédente est incomplète ; elle est conservée".into(),
        );
    }
    write_secret_atomic(&validation, b"")?;
    let dump_command = r#"exec mysqldump --user=root --password="$MYSQL_ROOT_PASSWORD" --all-databases --single-transaction --quick --routines --events --triggers --hex-blob --no-tablespaces --set-gtid-purged=OFF"#;
    runner.run_to_file(
        "docker",
        &compose_args(
            compose_file,
            &["exec", "-T", "database", "sh", "-c", dump_command],
        ),
        Some(server_root),
        &validation,
        error_log,
    )?;
    validate_database_backup(&validation)?;
    fs::remove_file(&validation).map_err(|error| error.to_string())
}

struct DockerLocalGuideSource<'a, R: CommandRunner> {
    runner: &'a R,
    compose_file: &'a Path,
    server_root: &'a Path,
    observed_at_unix_ms: Option<u64>,
}

impl<R: CommandRunner> DockerLocalGuideSource<'_, R> {
    fn rows(
        &self,
        sql_template: &str,
        term_hex: &HexEncodedGuideTerm,
        locale: LocalGuideLocale,
    ) -> Result<Option<LocalGuideTabularSnapshot>, LocalSourceError> {
        let locale = match locale {
            LocalGuideLocale::FrFr => "frFR",
            LocalGuideLocale::EnUs => "enUS",
        };
        let sql = sql_template.replace("__LOCALE__", locale);
        let shell_command = format!(
            "export MYSQL_PWD=\"$MYSQL_ROOT_PASSWORD\"; exec mysql --batch --raw --skip-column-names --connect-timeout=5 --user=root --database=acore_world --execute=\"{sql}\""
        );
        let mut args = compose_args(self.compose_file, &["exec", "-T", "-e"]);
        args.push(format!("REALMBOX_GUIDE_TERM_HEX={}", term_hex.as_str()).into());
        args.extend([
            "database".into(),
            "sh".into(),
            "-c".into(),
            shell_command.into(),
        ]);
        let rows = self
            .runner
            .run_bounded(
                "docker",
                &args,
                Some(self.server_root),
                LOCAL_GUIDE_QUERY_TIMEOUT,
            )
            .map_err(|_| LocalSourceError::Unavailable)?;
        Ok(Some(LocalGuideTabularSnapshot {
            rows,
            provenance: LocalProvenance {
                scope: LocalSourceScope::RuntimeSnapshot,
                source_id: "realmbox-world-catalog".into(),
                observed_at_unix_ms: self.observed_at_unix_ms,
            },
        }))
    }
}

impl<R: CommandRunner> LocalGuideSearchDataSource for DockerLocalGuideSource<'_, R> {
    fn quest_rows(
        &self,
        term_hex: &HexEncodedGuideTerm,
        locale: LocalGuideLocale,
    ) -> Result<Option<LocalGuideTabularSnapshot>, LocalSourceError> {
        self.rows(LOCAL_GUIDE_QUEST_SQL, term_hex, locale)
    }

    fn item_rows(
        &self,
        term_hex: &HexEncodedGuideTerm,
        locale: LocalGuideLocale,
    ) -> Result<Option<LocalGuideTabularSnapshot>, LocalSourceError> {
        self.rows(LOCAL_GUIDE_ITEM_SQL, term_hex, locale)
    }
}

fn docker_volume_exists<R: CommandRunner>(runner: &R, name: &str) -> bool {
    runner
        .run(
            "docker",
            &["volume".into(), "inspect".into(), name.into()],
            None,
        )
        .is_ok()
}

fn docker_volume_exists_bounded<R: CommandRunner>(
    runner: &R,
    name: &str,
    timeout: Duration,
) -> bool {
    runner
        .run_bounded(
            "docker",
            &["volume".into(), "inspect".into(), name.into()],
            None,
            timeout,
        )
        .is_ok()
}

fn compose_service_image(source: &str, service: &str) -> Option<String> {
    let service_header = format!("  {service}:");
    let mut in_service = false;
    for line in source.lines() {
        if line == service_header {
            in_service = true;
            continue;
        }
        if in_service && line.starts_with("  ") && !line.starts_with("    ") {
            break;
        }
        if in_service && let Some(image) = line.trim().strip_prefix("image:") {
            return Some(image.trim().to_owned());
        }
    }
    None
}

fn runtime_server_matches_images(
    server_root: &Path,
    images: &ServerImages,
) -> Result<bool, String> {
    let compose = fs::read_to_string(server_root.join("compose.realmbox.yaml"))
        .map_err(|error| error.to_string())?;
    Ok([
        ("server-data-init", images.tools.as_str()),
        ("db-import", images.db_import.as_str()),
        ("authserver", images.authserver.as_str()),
        ("worldserver", images.worldserver.as_str()),
    ]
    .into_iter()
    .all(|(service, expected)| {
        compose_service_image(&compose, service).as_deref() == Some(expected)
    }))
}

fn repair_missing_local_server_image<R: CommandRunner>(
    runner: &R,
    record: &InstallationRecord,
) -> Result<bool, String> {
    let images = embedded_server_images()?;
    repair_missing_local_server_image_with(runner, record, images.as_ref())
}

fn repair_missing_local_server_image_with<R: CommandRunner>(
    runner: &R,
    record: &InstallationRecord,
    replacement: Option<&ServerImages>,
) -> Result<bool, String> {
    let source = fs::read_to_string(&record.compose_file).map_err(|error| error.to_string())?;
    let Some(current_worldserver) = compose_service_image(&source, "worldserver") else {
        return Ok(false);
    };
    if validate_immutable_server_image(&current_worldserver).is_ok()
        || runner
            .run(
                "docker",
                &[
                    "image".into(),
                    "inspect".into(),
                    current_worldserver.clone().into(),
                ],
                None,
            )
            .is_ok()
    {
        return Ok(false);
    }
    let images = replacement.ok_or_else(|| {
        format!(
            "l’image locale {current_worldserver} a disparu et ce build de développement ne contient pas de remplacement immuable"
        )
    })?;
    let repaired = compose_file(
        "1000",
        "1000",
        &record.game_data_root,
        DEFAULT_DOCKER_BUILD_JOBS,
        Some(images),
    );
    write_atomic(&record.compose_file, repaired.as_bytes())?;
    Ok(true)
}

fn compose_args(compose_file: &Path, trailing: &[&str]) -> Vec<OsString> {
    assert!(
        !trailing
            .iter()
            .any(|argument| matches!(*argument, "--volumes" | "-v")),
        "RealmBox interdit la suppression de volumes persistants"
    );
    let mut args = vec![
        "compose".into(),
        "-p".into(),
        COMPOSE_PROJECT_NAME.into(),
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
      - ./mmaps-config.yaml:/azerothcore/env/dist/bin/mmaps-config.yaml:ro
    command:
      - bash
      - -c
      - >-
        set -euo pipefail;
        if ! grep -Fxq "REALMBOX_SOURCE_ID=$${REALMBOX_SOURCE_ID}" /work/extraction-version 2>/dev/null; then
          ln -sfn /client-data /work/Data;
          /azerothcore/env/dist/bin/map_extractor;
          rm -rf /work/Buildings;
          /azerothcore/env/dist/bin/vmap4_extractor;
          rm -rf /work/vmaps;
          mkdir -p /work/vmaps;
          /azerothcore/env/dist/bin/vmap4_assembler /work/Buildings /work/vmaps;
          rm -rf /work/mmaps;
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

    #[test]
    fn docker_resolution_falls_back_when_finder_path_omits_docker_desktop() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let finder_bin = temporary.path().join("finder-bin");
        let docker_desktop_bin = temporary.path().join("Docker.app/docker");
        fs::create_dir_all(&finder_bin).expect("Finder bin");
        fs::create_dir_all(docker_desktop_bin.parent().expect("Docker parent"))
            .expect("Docker Desktop bin");
        fs::write(&docker_desktop_bin, b"docker fixture").expect("Docker fixture");
        let finder_path = env::join_paths([finder_bin]).expect("Finder PATH");

        assert_eq!(
            resolve_program_with(
                "docker",
                Some(finder_path.as_os_str()),
                std::slice::from_ref(&docker_desktop_bin),
            ),
            docker_desktop_bin
        );
    }

    #[test]
    fn docker_resolution_prefers_the_configured_path() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let configured_bin = temporary.path().join("configured-bin");
        let configured_docker = configured_bin.join("docker");
        let fallback_docker = temporary.path().join("Docker.app/docker");
        fs::create_dir_all(&configured_bin).expect("configured bin");
        fs::create_dir_all(fallback_docker.parent().expect("Docker parent"))
            .expect("Docker Desktop bin");
        fs::write(&configured_docker, b"configured Docker").expect("configured Docker fixture");
        fs::write(&fallback_docker, b"fallback Docker").expect("fallback Docker fixture");
        let configured_path = env::join_paths([configured_bin]).expect("configured PATH");

        assert_eq!(
            resolve_program_with(
                "docker",
                Some(configured_path.as_os_str()),
                std::slice::from_ref(&fallback_docker),
            ),
            configured_docker
        );
    }

    #[test]
    fn docker_child_path_includes_desktop_credential_helpers() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let desktop_bin = temporary.path().join("Docker.app/Contents/Resources/bin");
        let docker = desktop_bin.join("docker");
        fs::create_dir_all(&desktop_bin).expect("docker resources");
        fs::write(&docker, b"docker fixture").expect("docker fixture");
        fs::write(
            desktop_bin.join("docker-credential-desktop"),
            b"credential helper fixture",
        )
        .expect("credential helper fixture");

        let path = docker_support_path(
            Some(OsStr::new("/usr/bin:/bin")),
            &docker,
            std::slice::from_ref(&docker),
        )
        .expect("child path");
        let directories = env::split_paths(&path).collect::<Vec<_>>();

        assert_eq!(directories.first(), Some(&desktop_bin));
        assert!(directories.contains(&PathBuf::from("/usr/bin")));
        assert!(directories.contains(&PathBuf::from("/bin")));
    }

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

    fn write_ollama_chat_fixture(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("module config dir");
        }
        fs::write(
            path,
            "OllamaChat.Enable = 0\nOllamaChat.Url = http://localhost\nOllamaChat.CapabilityProbeTimeoutSeconds = 10\nOllamaChat.HttpTimeoutSeconds = 120\nOllamaChat.Model = test\nOllamaChat.NumPredict = 1\nOllamaChat.ReasoningTokenReserve = 1\nOllamaChat.Temperature = 1\nOllamaChat.TopP = 1\nOllamaChat.NumCtx = 1\nOllamaChat.SystemPrompt = \"test\"\nOllamaChat.ChatPromptTemplate = \"test {player_message}\"\nOllamaChat.MaxConcurrentQueries = 0\nOllamaChat.WorkerThreads = 4\nOllamaChat.MaxQueueDepth = 64\nOllamaChat.ThinkMode = \"auto\"\nOllamaChat.ThinkModeEnableForModule = 1\nOllamaChat.DebugEnabled = 1\nOllamaChat.DebugShowFullPrompt = 1\nOllamaChat.EnableChatHistory = 1\nOllamaChat.ConversationHistorySaveInterval = 10\nOllamaChat.EnableChatBotSnapshotTemplate = 1\nOllamaChat.Memory.Enable = 1\nOllamaChat.Relationship.Enable = 1\nOllamaChat.EnableRAG = 1\nOllamaChat.EnableEmoteReactions = 1\nOllamaChat.PlayerReplyChance.Say = 1\nOllamaChat.PlayerReplyChance.Channel = 1\nOllamaChat.PlayerReplyChance.Party = 1\nOllamaChat.PlayerReplyChance.Guild = 1\nOllamaChat.BotReplyChance.Say = 1\nOllamaChat.BotReplyChance.Channel = 1\nOllamaChat.BotReplyChance.Party = 1\nOllamaChat.BotReplyChance.Guild = 1\nOllamaChat.EnableWhisperReplies = 0\nOllamaChat.MaxBotsToPick = 2\nOllamaChat.BotConversation.MaxChainDepth = 3\nOllamaChat.BotConversation.ChanceDecayPct = 50\nOllamaChat.BotConversation.RequireRecentHuman = 1\nOllamaChat.Cooldown.PerBotSeconds = 45\nOllamaChat.Cooldown.PerScopeSeconds = 15\nOllamaChat.RateLimit.ScopePerMinute = 8\nOllamaChat.RateLimit.GlobalPerMinute = 40\nOllamaChat.EnableRandomChatter = 0\nOllamaChat.MinRandomInterval = 45\nOllamaChat.MaxRandomInterval = 180\nOllamaChat.RandomChatterRealPlayerDistance = 200.0\nOllamaChat.RandomChatterPromptTemplate = \"test random\"\nOllamaChat.RandomChatterPromptVariations = \"test one|test two\"\nOllamaChat.RandomChatterBotCommentChance = 1\nOllamaChat.RandomChatterMaxBotsPerPlayer = 2\nOllamaChat.EnableEventChatter = 0\nOllamaChat.EventChatterBotCommentChance = 1\nOllamaChat.EventChatterBotSelfCommentChance = 1\nOllamaChat.EventChatterMaxBotsPerPlayer = 2\nOllamaChat.EventChatterPromptTemplate = \"test event\"\nOllamaChat.DisableRepliesInCombat = 0\nOllamaChat.DisableForCustomChannels = 0\nOllamaChat.DisableForGuild = 0\nOllamaChat.DisableForParty = 0\nOllamaChat.EnableSentimentTracking = 1\nOllamaChat.UnmanagedDefault = keep\n",
        )
        .expect("module config");
    }

    fn playerbots_fixture(enabled: bool, count: usize, extra: &str) -> String {
        let value = u8::from(enabled);
        format!(
            "AiPlayerbot.Enabled = {value}\nAiPlayerbot.RandomBotAutologin = {value}\nAiPlayerbot.MinRandomBots = {count}\nAiPlayerbot.MaxRandomBots = {count}\nAiPlayerbot.RandomBotGuildCount = 20\nAiPlayerbot.DisabledWithoutRealPlayer = 0\nAiPlayerbot.DisabledWithoutRealPlayerLoginDelay = 30\nAiPlayerbot.DisabledWithoutRealPlayerLogoutDelay = 300\nAiPlayerbot.BotActiveAlone = 10\nAiPlayerbot.BotActiveAloneForceWhenInRadius = 150\nAiPlayerbot.BotActiveAloneForceWhenInZone = 1\nAiPlayerbot.BotActiveAloneForceWhenInMap = 0\nAiPlayerbot.LevelBrackets.Enabled = 0\nAiPlayerbot.LevelBrackets.CheckFrequency = 300\nAiPlayerbot.LevelBrackets.Dynamic.UseDynamicDistribution = 0\nAiPlayerbot.LevelBrackets.Dynamic.RealPlayerWeight = 1.0\nAiPlayerbot.LevelBrackets.Dynamic.SyncFactions = 0\nAiPlayerbot.AutoTeleportForLevel = 1\nAiPlayerbot.RandomBotTeleLowerLevel = 1\nAiPlayerbot.RandomBotTeleHigherLevel = 3\nAiPlayerbot.MinRandomBotTeleportInterval = 3600\nAiPlayerbot.MaxRandomBotTeleportInterval = 18000\nAiPlayerbot.ProbTeleToBankers = 0.25\n{extra}"
        )
    }

    fn save_test_installation<R: CommandRunner>(service: &LauncherService<R>, root: &Path) {
        let runtime_root = root.join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::write(&compose_file, "services: {}\n").expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        service
            .save_record(&InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: root.join("game-source"),
                runtime_root,
                client_executable,
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
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");
    }

    #[derive(Default)]
    struct RecordingRunner {
        commands: Mutex<Vec<String>>,
        bounded_commands: Mutex<Vec<(String, Duration)>>,
        fail_next_input: Mutex<bool>,
        fail_next_ps: Mutex<bool>,
        empty_next_ps: Mutex<bool>,
        docker_memory: Mutex<Option<String>>,
        docker_volumes_present: Mutex<Option<bool>>,
        docker_images_present: Mutex<Option<bool>>,
        running_services: Mutex<Vec<String>>,
        local_guide_output: Mutex<Option<Result<String, String>>>,
        timeout_next_local_guide_query: Mutex<bool>,
        fail_next_database_start: Mutex<bool>,
        fail_next_database_stop: Mutex<bool>,
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
                let mut fail_next_ps = self.fail_next_ps.lock().expect("ps failure");
                if *fail_next_ps {
                    *fail_next_ps = false;
                    return Err("injected docker ps failure".into());
                }
                let mut empty_next_ps = self.empty_next_ps.lock().expect("empty ps");
                if *empty_next_ps {
                    *empty_next_ps = false;
                    return Ok(String::new());
                }
                if args.iter().any(|argument| argument == "--services") {
                    let service = args
                        .last()
                        .map(|argument| argument.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    return Ok(
                        if self
                            .running_services
                            .lock()
                            .expect("running services")
                            .contains(&service)
                        {
                            service
                        } else {
                            String::new()
                        },
                    );
                }
                return Ok("realmbox-worldserver-container".into());
            }
            if program == "docker"
                && args.first() == Some(&OsString::from("volume"))
                && args.get(1) == Some(&OsString::from("inspect"))
            {
                return if *self.docker_volumes_present.lock().expect("docker volumes")
                    == Some(false)
                {
                    Err("volume absent".into())
                } else {
                    Ok("{}".into())
                };
            }
            if program == "docker"
                && args.first() == Some(&OsString::from("image"))
                && args.get(1) == Some(&OsString::from("inspect"))
            {
                return if *self.docker_images_present.lock().expect("docker images") == Some(false)
                {
                    Err("image absente".into())
                } else {
                    Ok("{}".into())
                };
            }
            if program == "docker" && args.iter().any(|arg| arg == "inspect") {
                return Ok("true false".into());
            }
            if program == "docker" && args.iter().any(|arg| arg == "{{.MemTotal}}") {
                return Ok(self
                    .docker_memory
                    .lock()
                    .expect("docker memory")
                    .clone()
                    .unwrap_or_else(|| "34359738368".into()));
            }
            if args.iter().any(|arg| {
                arg.to_string_lossy()
                    .starts_with("REALMBOX_GUIDE_TERM_HEX=")
            }) {
                return self
                    .local_guide_output
                    .lock()
                    .expect("guide output")
                    .clone()
                    .unwrap_or_else(|| Ok(String::new()));
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
            if args.last() == Some(&OsString::from("database")) {
                let failure = if args.iter().any(|arg| arg == "up") {
                    Some(&self.fail_next_database_start)
                } else if args.iter().any(|arg| arg == "stop") {
                    Some(&self.fail_next_database_stop)
                } else {
                    None
                };
                if let Some(failure) = failure {
                    let mut failure = failure.lock().expect("database lifecycle failure");
                    if *failure {
                        *failure = false;
                        return Err("injected database lifecycle failure".into());
                    }
                }
            }
            Ok(())
        }
        fn run_bounded(
            &self,
            program: &str,
            args: &[OsString],
            current_dir: Option<&Path>,
            timeout: Duration,
        ) -> Result<String, String> {
            self.bounded_commands
                .lock()
                .expect("bounded commands")
                .push((
                    format!(
                        "{program} {}",
                        args.iter()
                            .map(|arg| arg.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(" ")
                    ),
                    timeout,
                ));
            if args.iter().any(|arg| {
                arg.to_string_lossy()
                    .starts_with("REALMBOX_GUIDE_TERM_HEX=")
            }) {
                let mut timeout_next = self
                    .timeout_next_local_guide_query
                    .lock()
                    .expect("guide timeout");
                if *timeout_next {
                    *timeout_next = false;
                    return Err("injected bounded command timeout".into());
                }
            }
            self.run(program, args, current_dir)
        }
        fn run_long_bounded(
            &self,
            program: &str,
            args: &[OsString],
            current_dir: Option<&Path>,
            log_path: &Path,
            timeout: Duration,
        ) -> Result<(), String> {
            self.bounded_commands
                .lock()
                .expect("bounded commands")
                .push((
                    format!(
                        "{program} {}",
                        args.iter()
                            .map(|arg| arg.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(" ")
                    ),
                    timeout,
                ));
            self.run_long(program, args, current_dir, log_path)
        }
        fn run_to_file(
            &self,
            program: &str,
            args: &[OsString],
            _current_dir: Option<&Path>,
            output_path: &Path,
            _error_path: &Path,
        ) -> Result<(), String> {
            self.commands.lock().expect("commands").push(format!(
                "{program} {} > {}",
                args.iter()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" "),
                output_path.display()
            ));
            if args.iter().any(|argument| {
                argument == "/azerothcore/env/ref/etc/modules/mod_ollama_chat.conf.dist"
            }) {
                write_ollama_chat_fixture(output_path);
                return Ok(());
            }
            fs::write(
                output_path,
                "-- Current Database: `acore_auth`\n-- Current Database: `acore_characters`\n-- Current Database: `acore_playerbots`\n-- Current Database: `acore_world`\n",
            )
            .map_err(|error| error.to_string())
        }
        fn run_with_input(
            &self,
            program: &str,
            args: &[OsString],
            _current_dir: Option<&Path>,
            input_path: &Path,
            _log_path: &Path,
        ) -> Result<(), String> {
            self.commands.lock().expect("commands").push(format!(
                "{program} {} < {}",
                args.iter()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" "),
                input_path.display()
            ));
            let mut fail_next_input = self.fail_next_input.lock().expect("input failure");
            if *fail_next_input {
                *fail_next_input = false;
                return Err("injected database import failure".into());
            }
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
        fn wait_service_tcp(
            &self,
            _compose_file: &Path,
            service: &str,
            port: u16,
            _timeout: Duration,
        ) -> Result<(), String> {
            self.commands
                .lock()
                .expect("commands")
                .push(format!("wait-service {service}:{port}"));
            Ok(())
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
    fn local_guide_uses_fixed_read_only_sql_and_cleans_up_its_database() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::default();
        *runner.local_guide_output.lock().expect("guide output") =
            Some(Ok("17\t54657374\t46616374\t5\t7175657374".into()));
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            runner,
        )
        .expect("service");
        save_test_installation(&service, temporary.path());
        let query = LocalGuideQuery::new(
            crate::local_guide::LocalGuideKind::Quest,
            "L’épreuve",
            LocalGuideLocale::FrFr,
        )
        .expect("query");
        let response = service.query_local_guide(query).expect("guide");
        assert_eq!(response.entries[0].title, "Test");
        let commands = service.runner.commands.lock().expect("commands");
        let commands = commands.join("\n");
        assert!(commands.contains(
            "up -d --no-build --pull never --no-deps --wait --wait-timeout 120 database"
        ));
        assert!(commands.contains("START TRANSACTION READ ONLY"));
        assert!(commands.contains("MAX_EXECUTION_TIME(2000)"));
        assert!(commands.contains("LIMIT 8; COMMIT"));
        assert!(commands.contains("quest_template_locale"));
        assert!(commands.contains("l.locale = 'frFR'"));
        assert!(commands.contains("REALMBOX_GUIDE_TERM_HEX="));
        assert!(commands.contains("export MYSQL_PWD=\"$MYSQL_ROOT_PASSWORD\""));
        assert!(!commands.contains("--password="));
        assert!(!commands.contains("L’épreuve"));
        assert!(commands.contains("stop database"));
        for forbidden in [
            "db-import",
            "DROP ",
            "DELETE ",
            "INSERT ",
            "UPDATE ",
            "worldserver",
        ] {
            assert!(!commands.contains(forbidden));
        }
        let bounded = service
            .runner
            .bounded_commands
            .lock()
            .expect("bounded commands");
        assert_eq!(bounded.len(), 5);
        for ((command, timeout), (expected, seconds)) in bounded.iter().zip([
            ("volume inspect", 10),
            ("ps --status running --services database", 10),
            ("up -d", 125),
            ("REALMBOX_GUIDE_TERM_HEX=", 10),
            ("stop database", 15),
        ]) {
            assert!(command.contains(expected));
            assert_eq!(*timeout, Duration::from_secs(seconds));
        }
    }

    #[test]
    fn local_guide_never_starts_a_missing_player_volume() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::default();
        *runner.docker_volumes_present.lock().expect("volumes") = Some(false);
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            runner,
        )
        .expect("service");
        save_test_installation(&service, temporary.path());
        let response = service
            .query_local_guide(
                LocalGuideQuery::new(
                    crate::local_guide::LocalGuideKind::Item,
                    "Sword",
                    LocalGuideLocale::EnUs,
                )
                .expect("query"),
            )
            .expect("unavailable guide");
        assert!(response.entries.is_empty());
        let commands = service.runner.commands.lock().expect("commands").join("\n");
        assert!(commands.contains("volume inspect"));
        assert!(!commands.contains(" up "));
        assert!(!commands.contains("mysql"));
    }

    #[test]
    fn local_guide_preserves_an_online_database_and_fails_quietly() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::default();
        runner
            .running_services
            .lock()
            .expect("services")
            .push("database".into());
        *runner.local_guide_output.lock().expect("guide output") =
            Some(Err("private SQL error".into()));
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            runner,
        )
        .expect("service");
        save_test_installation(&service, temporary.path());
        let response = service
            .query_local_guide(
                LocalGuideQuery::new(
                    crate::local_guide::LocalGuideKind::Item,
                    "Sword",
                    LocalGuideLocale::EnUs,
                )
                .expect("query"),
            )
            .expect("quiet fallback");
        assert!(response.entries.is_empty());
        assert!(
            !serde_json::to_string(&response)
                .expect("response")
                .contains("private")
        );
        let commands = service.runner.commands.lock().expect("commands").join("\n");
        assert!(commands.contains("item_template_locale"));
        assert!(commands.contains("l.locale = 'enUS'"));
        assert!(!commands.contains(" up "));
        assert!(!commands.contains(" stop "));
    }

    fn write_solo_config_fixture(root: &Path) -> PathBuf {
        let path = root
            .join(RUNTIME_DIRECTORY)
            .join("server/env/dist/etc/worldserver.conf");
        fs::create_dir_all(path.parent().expect("config dir")).expect("config dir");
        let catalog = crate::solo_profiles::ProfileCatalog::realm_box_v1().expect("catalog");
        let values = &catalog
            .definition(SoloProfile::Normal)
            .expect("normal profile")
            .values;
        let mut text = "# fixture\nUnmanaged.Option = keep\n".to_string();
        for (key, value) in values {
            text.push_str(&format!(
                "{} = {}\n",
                key.config_key(),
                value.config_value()
            ));
        }
        fs::write(&path, text).expect("config fixture");
        path
    }

    #[test]
    fn solo_profile_commands_are_reversible_without_database_or_runtime_start() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");
        save_test_installation(&service, temporary.path());
        let config = write_solo_config_fixture(temporary.path());
        let before = fs::read_to_string(&config).expect("before");
        let result = service
            .configure_solo_profile(SoloProfile::Comfortable)
            .expect("profile");
        assert_eq!(result.active_profile, Some(SoloProfile::Comfortable));
        assert!(result.rollback_available);
        let changed = fs::read_to_string(&config).expect("changed");
        assert!(changed.contains("Rate.XP.Kill = 2"));
        assert!(changed.contains("MaxPrimaryTradeSkill = 11"));
        assert!(changed.contains("Unmanaged.Option = keep"));
        let restored = service.rollback_solo_profile().expect("rollback");
        assert_eq!(restored.active_profile, Some(SoloProfile::Normal));
        assert_eq!(fs::read_to_string(&config).expect("restored"), before);
        let commands = service.runner.commands.lock().expect("commands").join("\n");
        assert!(commands.contains("ps --status running --services worldserver"));
        for forbidden in [" up ", "mysql", "db-import", " down ", " stop "] {
            assert!(!commands.contains(forbidden));
        }
    }

    #[test]
    fn solo_profile_commands_refuse_a_running_world_before_any_write() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::default();
        runner
            .running_services
            .lock()
            .expect("services")
            .push("worldserver".into());
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            runner,
        )
        .expect("service");
        save_test_installation(&service, temporary.path());
        let config = write_solo_config_fixture(temporary.path());
        let before = fs::read(&config).expect("before");
        assert!(
            service
                .configure_solo_profile(SoloProfile::Accelerated)
                .is_err()
        );
        assert!(service.rollback_solo_profile().is_err());
        assert_eq!(fs::read(&config).expect("after"), before);
        assert!(!temporary.path().join("solo-profiles-v1").exists());
    }

    #[test]
    fn local_guide_closes_a_temporarily_started_database_after_query_failure() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::default();
        *runner.local_guide_output.lock().expect("guide output") =
            Some(Err("query unavailable".into()));
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            runner,
        )
        .expect("service");
        save_test_installation(&service, temporary.path());
        service
            .query_local_guide(
                LocalGuideQuery::new(
                    crate::local_guide::LocalGuideKind::Quest,
                    "Test",
                    LocalGuideLocale::EnUs,
                )
                .expect("query"),
            )
            .expect("quiet fallback");
        let commands = service.runner.commands.lock().expect("commands").join("\n");
        assert!(commands.contains(
            "up -d --no-build --pull never --no-deps --wait --wait-timeout 120 database"
        ));
        assert!(commands.contains("stop database"));
    }

    #[test]
    fn local_guide_query_timeout_still_attempts_bounded_database_cleanup() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::default();
        *runner
            .timeout_next_local_guide_query
            .lock()
            .expect("guide timeout") = true;
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            runner,
        )
        .expect("service");
        save_test_installation(&service, temporary.path());
        let response = service
            .query_local_guide(
                LocalGuideQuery::new(
                    crate::local_guide::LocalGuideKind::Quest,
                    "Test",
                    LocalGuideLocale::EnUs,
                )
                .expect("query"),
            )
            .expect("quiet timeout fallback");
        assert_eq!(
            response.uncertainty,
            crate::local_guide::LocalGuideUncertainty::Unavailable
        );
        let bounded = service
            .runner
            .bounded_commands
            .lock()
            .expect("bounded commands");
        assert_eq!(bounded.len(), 5);
        assert!(bounded[3].0.contains("REALMBOX_GUIDE_TERM_HEX="));
        assert_eq!(bounded[3].1, Duration::from_secs(10));
        assert!(bounded[4].0.contains("stop database"));
        assert_eq!(bounded[4].1, Duration::from_secs(15));
    }

    #[test]
    fn local_guide_cleans_up_a_partial_start_and_reports_cleanup_failure() {
        for stop_fails in [false, true] {
            let temporary = tempfile::tempdir().expect("tempdir");
            let runner = RecordingRunner::default();
            *runner
                .fail_next_database_start
                .lock()
                .expect("start failure") = true;
            *runner.fail_next_database_stop.lock().expect("stop failure") = stop_fails;
            let service = LauncherService::new(
                temporary.path().to_path_buf(),
                temporary.path().join("addon"),
                runner,
            )
            .expect("service");
            save_test_installation(&service, temporary.path());
            let error = service
                .query_local_guide(
                    LocalGuideQuery::new(
                        crate::local_guide::LocalGuideKind::Quest,
                        "Test",
                        LocalGuideLocale::EnUs,
                    )
                    .expect("query"),
                )
                .expect_err("partial start must fail");
            assert_eq!(error.contains("arrêt de la base à vérifier"), stop_fails);
            let commands = service.runner.commands.lock().expect("commands").join("\n");
            assert!(commands.contains(
                "up -d --no-build --pull never --no-deps --wait --wait-timeout 120 database"
            ));
            assert!(commands.contains("stop database"));
            assert!(!commands.contains("REALMBOX_GUIDE_TERM_HEX="));
            assert!(!commands.contains("db-import"));
        }
    }

    #[test]
    fn ambient_dialogue_language_follows_the_client_locale() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let french = temporary.path().join("French");
        let english = temporary.path().join("English");
        write_complete_game_data(&french, "frFR");
        write_complete_game_data(&english, "enGB");

        assert_eq!(
            dialogue_language_for_game_data(&french),
            DialogueLanguage::French
        );
        assert_eq!(
            dialogue_language_for_game_data(&english),
            DialogueLanguage::English
        );
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
            fs::read_to_string(backup.join(source_id(&game)).join("realmlist-frFR.wtf"))
                .expect("backup"),
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
    fn changes_managed_game_data_path_without_touching_the_server_runtime() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let old_game = temporary.path().join("Wrath old");
        let new_game = temporary.path().join("Wrath new");
        let addon = temporary.path().join("addon");
        for game in [&old_game, &new_game] {
            write_complete_game_data(game, "frFR");
            fs::write(
                game.join("Data/frFR/realmlist.wtf"),
                "set realmlist example.invalid\n",
            )
            .expect("realmlist");
        }
        fs::create_dir_all(&addon).expect("addon");
        for filename in [
            "RealmBoxCompanions.lua",
            "RealmBoxCompanions.toc",
            "RealmBoxCompanions.xml",
        ] {
            fs::write(addon.join(filename), filename).expect("addon fixture");
        }

        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::write(&compose_file, "services: {}\n").expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        prepare_managed_openwow_game(
            &RecordingRunner::default(),
            &old_game,
            &runtime_root.join("game"),
            &addon,
            &temporary.path().join("original-client-backup"),
        )
        .expect("initial managed game");

        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            addon,
            RecordingRunner::default(),
        )
        .expect("service");
        service
            .save_record(&InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: fs::canonicalize(&old_game).expect("old game"),
                runtime_root: runtime_root.clone(),
                client_executable,
                client_choice: ClientChoice::ManagedOpenWow,
                compose_file: compose_file.clone(),
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
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");

        let status = service
            .change_game_data_path(&new_game)
            .expect("change game path");

        let canonical_new = fs::canonicalize(&new_game).expect("new game");
        assert_eq!(status.phase, LauncherPhase::Ready);
        assert_eq!(
            status.game_data_path,
            Some(canonical_new.display().to_string())
        );
        assert_eq!(
            service
                .load_record()
                .expect("record")
                .expect("installed")
                .game_data_root,
            canonical_new
        );
        assert_eq!(
            fs::canonicalize(runtime_root.join("game/Data/common.MPQ")).expect("managed link"),
            fs::canonicalize(new_game.join("Data/common.MPQ")).expect("new common")
        );
        assert_eq!(
            fs::read_to_string(compose_file).expect("compose preserved"),
            "services: {}\n"
        );
        assert!(!runtime_root.join(".game-path-previous").exists());
    }

    #[test]
    fn rejects_game_data_path_change_while_the_owned_client_is_running() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let new_game = temporary.path().join("Wrath new");
        write_complete_game_data(&new_game, "frFR");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::write(&compose_file, "services: {}\n").expect("compose");
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
                game_data_root: temporary.path().join("old game"),
                runtime_root,
                client_executable,
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
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");
        service.client_process_id = Some(42);

        assert!(
            service
                .change_game_data_path(&new_game)
                .expect_err("running client must block change")
                .contains("arrêtez le monde")
        );
    }

    #[test]
    fn changes_original_windows_client_executable_and_keeps_a_source_specific_backup() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let new_game = temporary.path().join("Wrath new");
        let addon = temporary.path().join("addon");
        write_complete_game_data(&new_game, "frFR");
        fs::write(new_game.join("Wow.exe"), "user binary").expect("client");
        fs::write(
            new_game.join("Data/frFR/realmlist.wtf"),
            "set realmlist another.example\n",
        )
        .expect("realmlist");
        fs::create_dir_all(&addon).expect("addon");
        for filename in [
            "RealmBoxCompanions.lua",
            "RealmBoxCompanions.toc",
            "RealmBoxCompanions.xml",
        ] {
            fs::write(addon.join(filename), filename).expect("addon fixture");
        }

        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let old_client = temporary.path().join("Wrath old/Wow.exe");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::write(&compose_file, "services: {}\n").expect("compose");
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            addon,
            RecordingRunner::default(),
        )
        .expect("service");
        service
            .save_record(&InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: temporary.path().join("Wrath old"),
                runtime_root,
                client_executable: old_client,
                client_choice: ClientChoice::OriginalWindows,
                compose_file,
                bots_enabled: true,
                bot_count: 50,
                ai_enabled: false,
                ai_model: None,
                ollama_executable: None,
                client_sha256: None,
                ollama_sha256: None,
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: None,
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");

        service
            .change_game_data_path(&new_game)
            .expect("change original client");

        let canonical_new = fs::canonicalize(&new_game).expect("new game");
        let record = service.load_record().expect("record").expect("installed");
        assert_eq!(record.game_data_root, canonical_new);
        assert_eq!(record.client_executable, canonical_new.join("Wow.exe"));
        assert_eq!(
            fs::read_to_string(
                temporary
                    .path()
                    .join("original-client-backup")
                    .join(source_id(&canonical_new))
                    .join("realmlist-frFR.wtf")
            )
            .expect("source-specific backup"),
            "set realmlist another.example\n"
        );
        assert_eq!(
            fs::read_to_string(new_game.join("Data/frFR/realmlist.wtf")).expect("local realmlist"),
            "set realmlist 127.0.0.1\n"
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
        assert!(
            compose.contains("./mmaps-config.yaml:/azerothcore/env/dist/bin/mmaps-config.yaml:ro")
        );
        assert!(compose.contains("map_extractor"));
        assert!(compose.contains("rm -rf /work/Buildings;"));
        assert!(compose.contains("rm -rf /work/vmaps;"));
        assert!(compose.contains("rm -rf /work/mmaps;"));
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
    fn legacy_compose_receives_the_pinned_mmaps_configuration_mount() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let compose_path = temporary.path().join("compose.realmbox.yaml");
        let legacy = compose_file("501", "20", Path::new("/Games/Wrath"), 2, None).replace(
            "      - ./mmaps-config.yaml:/azerothcore/env/dist/bin/mmaps-config.yaml:ro\n",
            "",
        );
        fs::write(&compose_path, legacy).expect("legacy compose");

        ensure_mmaps_config_mount(&compose_path).expect("migration");
        ensure_mmaps_config_mount(&compose_path).expect("idempotent migration");

        let updated = fs::read_to_string(compose_path).expect("updated compose");
        assert_eq!(
            updated
                .matches("./mmaps-config.yaml:/azerothcore/env/dist/bin/mmaps-config.yaml:ro")
                .count(),
            1
        );
    }

    #[test]
    fn interrupted_server_data_extraction_is_made_restartable() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let compose_path = temporary.path().join("compose.realmbox.yaml");
        let legacy = compose_file("501", "20", Path::new("/Games/Wrath"), 2, None)
            .replace("          rm -rf /work/Buildings;\n", "")
            .replace("          rm -rf /work/vmaps;\n", "")
            .replace("          rm -rf /work/mmaps;\n", "");
        fs::write(&compose_path, legacy).expect("legacy compose");

        ensure_restartable_server_data_extraction(&compose_path).expect("migration");
        ensure_restartable_server_data_extraction(&compose_path).expect("idempotent migration");

        let updated = fs::read_to_string(compose_path).expect("updated compose");
        let buildings = updated.find("rm -rf /work/Buildings").expect("buildings");
        let extractor = updated.find("vmap4_extractor").expect("extractor");
        let vmaps = updated.find("rm -rf /work/vmaps").expect("vmaps");
        let assembler = updated.find("vmap4_assembler").expect("assembler");
        let mmaps = updated.find("rm -rf /work/mmaps").expect("mmaps");
        let generator = updated.find("mmaps_generator").expect("generator");
        assert!(buildings < extractor);
        assert!(vmaps < assembler);
        assert!(mmaps < generator);
    }

    #[test]
    fn partial_restart_cleanup_is_upgraded_to_include_buildings() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let compose_path = temporary.path().join("compose.realmbox.yaml");
        let partial = compose_file("501", "20", Path::new("/Games/Wrath"), 2, None)
            .replace("          rm -rf /work/Buildings;\n", "")
            .replace("          rm -rf /work/vmaps;\n", "")
            .replace("          rm -rf /work/mmaps;\n", "")
            .replace(
                "          /azerothcore/env/dist/bin/vmap4_extractor;\n",
                "          rm -rf /work/vmaps /work/mmaps;\n          /azerothcore/env/dist/bin/vmap4_extractor;\n",
            );
        fs::write(&compose_path, partial).expect("partial cleanup compose");

        ensure_restartable_server_data_extraction(&compose_path).expect("upgrade cleanup");

        let updated = fs::read_to_string(compose_path).expect("updated compose");
        assert!(updated.contains("rm -rf /work/Buildings;"));
        assert!(updated.contains("rm -rf /work/vmaps;"));
        assert!(updated.contains("rm -rf /work/mmaps;"));
        assert!(!updated.contains("          rm -rf /work/vmaps /work/mmaps;\n"));
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
    fn missing_local_worldserver_image_is_replaced_by_the_immutable_release_set() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let compose_path = temporary.path().join("server/compose.realmbox.yaml");
        fs::create_dir_all(compose_path.parent().expect("server")).expect("server");
        fs::write(
            &compose_path,
            compose_file(
                "1000",
                "1000",
                Path::new("/Games/Wrath"),
                2,
                Some(&ServerImages {
                    authserver: format!("ghcr.io/realmbox/auth@sha256:{}", "a".repeat(64)),
                    worldserver: format!("ghcr.io/realmbox/world@sha256:{}", "b".repeat(64)),
                    db_import: format!("ghcr.io/realmbox/db@sha256:{}", "c".repeat(64)),
                    tools: format!("ghcr.io/realmbox/tools@sha256:{}", "d".repeat(64)),
                }),
            )
            .replace(
                &format!("ghcr.io/realmbox/world@sha256:{}", "b".repeat(64)),
                "realmbox/worldserver:local",
            ),
        )
        .expect("local compose");
        let replacement = ServerImages {
            authserver: format!("ghcr.io/realmbox/auth@sha256:{}", "1".repeat(64)),
            worldserver: format!("ghcr.io/realmbox/world@sha256:{}", "2".repeat(64)),
            db_import: format!("ghcr.io/realmbox/db@sha256:{}", "3".repeat(64)),
            tools: format!("ghcr.io/realmbox/tools@sha256:{}", "4".repeat(64)),
        };
        let record = InstallationRecord {
            schema_version: INSTALL_SCHEMA,
            game_data_root: PathBuf::from("/Games/Wrath"),
            runtime_root: temporary.path().to_path_buf(),
            client_executable: temporary.path().join("client"),
            client_choice: ClientChoice::ManagedOpenWow,
            compose_file: compose_path.clone(),
            bots_enabled: true,
            bot_count: 50,
            ai_enabled: false,
            ai_model: None,
            ollama_executable: None,
            client_sha256: None,
            ollama_sha256: None,
            server_commit: SERVER_COMMIT.into(),
            playerbots_commit: PLAYERBOTS_COMMIT.into(),
            ollama_chat_commit: None,
            runtime_release: Some(runtime_release_id()),
        };
        let runner = RecordingRunner {
            docker_images_present: Mutex::new(Some(false)),
            ..Default::default()
        };

        assert!(
            repair_missing_local_server_image_with(&runner, &record, Some(&replacement))
                .expect("compose repair")
        );
        let repaired = fs::read_to_string(compose_path).expect("repaired compose");
        assert!(repaired.contains(&replacement.worldserver));
        assert!(repaired.contains(&replacement.authserver));
        assert!(!repaired.contains("realmbox/worldserver:local"));
        assert!(!repaired.contains("build:"));
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
            playerbots_fixture(false, 0, "AiPlayerbot.UnmanagedDefault = 42\n"),
        )
        .expect("source config");
        write_playerbots_config(temporary.path(), true, 50, BotPresence::Natural)
            .expect("playerbots config");
        let config = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/playerbots.conf"),
        )
        .expect("config");
        assert!(config.contains("AiPlayerbot.Enabled = 1"));
        assert!(config.contains("AiPlayerbot.MaxRandomBots = 50"));
        assert!(config.contains("AiPlayerbot.RandomBotGuildCount = 0"));
        assert!(config.contains("AiPlayerbot.DisabledWithoutRealPlayer = 1"));
        assert!(config.contains("AiPlayerbot.BotActiveAlone = 10"));
        assert!(config.contains("AiPlayerbot.BotActiveAloneForceWhenInZone = 0"));
        assert!(config.contains("AiPlayerbot.LevelBrackets.Enabled = 1"));
        assert!(config.contains("AiPlayerbot.LevelBrackets.CheckFrequency = 60"));
        assert!(config.contains("AiPlayerbot.LevelBrackets.Dynamic.RealPlayerWeight = 5.0"));
        assert!(config.contains("AiPlayerbot.AutoTeleportForLevel = 1"));
        assert!(config.contains("AiPlayerbot.MinRandomBotTeleportInterval = 1800"));
        assert!(config.contains("AiPlayerbot.MaxRandomBotTeleportInterval = 7200"));
        assert!(config.contains("AiPlayerbot.UnmanagedDefault = 42"));
        assert!(
            !temporary
                .path()
                .join("env/dist/etc/playerbots.conf")
                .exists()
        );
    }

    #[test]
    fn presence_configuration_targets_a_safe_same_faction_majority() {
        let temporary = tempfile::tempdir().expect("tempdir");
        install_realmbox_presence_config(&temporary.path().join("addon"), temporary.path())
            .expect("presence dist");
        write_realmbox_presence_config(temporary.path(), true, BotPresence::Close)
            .expect("presence config");

        let config = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/realmbox-presence.conf"),
        )
        .expect("presence config");
        assert!(config.contains("RealmBoxPresence.Enabled = 1"));
        assert!(config.contains("RealmBoxPresence.ScanIntervalMs = 1000"));
        assert!(config.contains("RealmBoxPresence.PlayerCooldownSeconds = 0"));
        assert!(config.contains("RealmBoxPresence.TargetFraction = 0.60"));
        assert!(config.contains("RealmBoxPresence.MaxBotsPerPlayer = 30"));
        assert!(config.contains("RealmBoxPresence.NearbyRadius = 150.0"));
        assert!(config.contains("RealmBoxPresence.SpawnMinRadius = 50.0"));
        assert!(config.contains("RealmBoxPresence.SpawnMaxRadius = 110.0"));
        assert!(config.contains("RealmBoxPresence.MaxMovesPerScan = 1"));
        assert!(config.contains("RealmBoxPresence.BotCooldownSeconds = 60"));
        assert!(config.contains("RealmBoxPresence.AutonomyReturnSeconds = 900"));
        assert!(config.contains("RealmBoxPresence.ReleasedBotGraceSeconds = 300"));
    }

    #[test]
    fn presence_profiles_keep_population_and_relocation_independent() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let source = temporary.path().join("modules/mod-playerbots/conf");
        fs::create_dir_all(&source).expect("source dir");
        fs::write(
            source.join("playerbots.conf.dist"),
            playerbots_fixture(false, 0, ""),
        )
        .expect("source config");
        install_realmbox_presence_config(&temporary.path().join("addon"), temporary.path())
            .expect("presence dist");

        write_playerbots_config(temporary.path(), true, 100, BotPresence::Dispersed)
            .expect("dispersed playerbots config");
        write_realmbox_presence_config(temporary.path(), true, BotPresence::Dispersed)
            .expect("dispersed presence config");
        let playerbots = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/playerbots.conf"),
        )
        .expect("playerbots config");
        let presence = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/realmbox-presence.conf"),
        )
        .expect("presence config");
        assert!(playerbots.contains("AiPlayerbot.MaxRandomBots = 100"));
        assert!(playerbots.contains("AiPlayerbot.BotActiveAlone = 5"));
        assert!(playerbots.contains("AiPlayerbot.LevelBrackets.Dynamic.RealPlayerWeight = 1.0"));
        assert!(presence.contains("RealmBoxPresence.Enabled = 0"));
        assert!(presence.contains("RealmBoxPresence.TargetFraction = 0.0"));
        assert!(presence.contains("RealmBoxPresence.AutonomyReturnSeconds = 60"));

        write_playerbots_config(temporary.path(), true, 100, BotPresence::Natural)
            .expect("natural playerbots config");
        write_realmbox_presence_config(temporary.path(), true, BotPresence::Natural)
            .expect("natural presence config");
        let presence = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/realmbox-presence.conf"),
        )
        .expect("presence config");
        assert!(presence.contains("RealmBoxPresence.Enabled = 1"));
        assert!(presence.contains("RealmBoxPresence.TargetFraction = 0.30"));
        assert!(presence.contains("RealmBoxPresence.MinBotsPerPlayer = 3"));
        assert!(presence.contains("RealmBoxPresence.MaxBotsPerPlayer = 15"));
        assert!(presence.contains("RealmBoxPresence.AutonomyReturnSeconds = 600"));
    }

    #[test]
    fn presence_config_upgrade_adds_the_new_key_once_without_overwriting_preferences() {
        let temporary = tempfile::tempdir().expect("tempdir");
        install_realmbox_presence_config(&temporary.path().join("addon"), temporary.path())
            .expect("presence dist");
        let destination = temporary
            .path()
            .join("env/dist/etc/modules/realmbox-presence.conf");
        let distributed = fs::read_to_string(destination.with_extension("conf.dist"))
            .expect("presence dist contents");
        let legacy = distributed
            .lines()
            .filter(|line| !line.starts_with("RealmBoxPresence.AutonomyReturnSeconds ="))
            .map(|line| {
                if line.starts_with("RealmBoxPresence.MaxLevelDelta =") {
                    "RealmBoxPresence.MaxLevelDelta = 7"
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(&destination, legacy).expect("legacy config");

        write_realmbox_presence_config(temporary.path(), true, BotPresence::Natural)
            .expect("upgraded config");
        write_realmbox_presence_config(temporary.path(), true, BotPresence::Natural)
            .expect("idempotent upgraded config");
        let upgraded = fs::read_to_string(destination).expect("upgraded config contents");
        assert_eq!(
            upgraded
                .lines()
                .filter(|line| line.starts_with("RealmBoxPresence.AutonomyReturnSeconds ="))
                .count(),
            1
        );
        assert!(upgraded.contains("RealmBoxPresence.AutonomyReturnSeconds = 600"));
        assert!(upgraded.contains("RealmBoxPresence.MaxLevelDelta = 7"));
    }

    #[test]
    fn fresh_worlds_default_to_natural_presence_and_legacy_preferences_stay_close() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");
        assert_eq!(
            service.world_preferences(None).bot_presence,
            BotPresence::Natural
        );

        fs::write(
            temporary.path().join("world-preferences.json"),
            r#"{"botsEnabled":true,"requestedBotCount":100}"#,
        )
        .expect("legacy preferences");
        let legacy = service.world_preferences(None);
        assert!(legacy.bots_enabled);
        assert_eq!(legacy.requested_bot_count, 100);
        assert_eq!(legacy.bot_presence, BotPresence::Close);
    }

    #[test]
    fn mmaps_configuration_is_copied_from_the_pinned_resource() {
        let temporary = tempfile::tempdir().expect("tempdir");
        install_mmaps_config(&temporary.path().join("addon"), temporary.path())
            .expect("mmaps config");
        let config =
            fs::read_to_string(temporary.path().join("mmaps-config.yaml")).expect("copied config");
        assert!(config.contains(SERVER_COMMIT));
        assert!(config.contains("verticesPerMapEdge: 2000"));
        assert!(config.contains("walkableSlopeAngle: 45"));
        assert!(config.contains("debugOutput: false"));
    }

    #[test]
    fn prebuilt_module_configuration_uses_the_image_dist_file() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let modules = temporary.path().join("env/dist/etc/modules");
        fs::create_dir_all(&modules).expect("module dir");
        fs::write(
            modules.join("playerbots.conf.dist"),
            playerbots_fixture(false, 0, "AiPlayerbot.ImageDefault = keep\n"),
        )
        .expect("image dist config");

        write_playerbots_config(temporary.path(), true, 5, BotPresence::Natural)
            .expect("prebuilt config");

        let config = fs::read_to_string(modules.join("playerbots.conf")).expect("config");
        assert!(config.contains("AiPlayerbot.Enabled = 1"));
        assert!(config.contains("AiPlayerbot.ImageDefault = keep"));
    }

    #[test]
    fn ollama_chat_is_local_bounded_and_allowlisted() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let source = temporary.path().join("modules/mod-ollama-chat/conf");
        fs::create_dir_all(&source).expect("source dir");
        write_ollama_chat_fixture(&source.join("mod_ollama_chat.conf.dist"));
        write_ollama_chat_config(
            temporary.path(),
            true,
            Some("qwen3:8b"),
            DialogueChattiness::Balanced,
            DialogueLanguage::English,
        )
        .expect("valid model");
        let config = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/mod_ollama_chat.conf"),
        )
        .expect("config");
        assert!(config.contains("http://host.docker.internal:11435/api/generate"));
        assert!(config.contains("OllamaChat.CapabilityProbeTimeoutSeconds = 5"));
        assert!(config.contains("OllamaChat.HttpTimeoutSeconds = 120"));
        assert!(config.contains("OllamaChat.MaxConcurrentQueries = 1"));
        assert!(config.contains("OllamaChat.WorkerThreads = 1"));
        assert!(config.contains("OllamaChat.MaxQueueDepth = 4"));
        assert!(config.contains("OllamaChat.PlayerReplyChance.Party = 100"));
        assert!(config.contains("OllamaChat.BotReplyChance.Say = 20"));
        assert!(config.contains("OllamaChat.BotReplyChance.Party = 50"));
        assert!(config.contains("OllamaChat.BotConversation.MaxChainDepth = 2"));
        assert!(config.contains("OllamaChat.BotConversation.ChanceDecayPct = 100"));
        assert!(config.contains("OllamaChat.Cooldown.PerScopeSeconds = 0"));
        assert!(config.contains("OllamaChat.RateLimit.GlobalPerMinute = 4"));
        assert!(config.contains("OllamaChat.EnableRandomChatter = 1"));
        assert!(config.contains("OllamaChat.MinRandomInterval = 90"));
        assert!(config.contains("OllamaChat.DisableForParty = 0"));
        assert!(config.contains("OllamaChat.DisableForCustomChannels = 1"));
        assert!(config.contains("exactly the language of the quoted player message"));
        assert!(config.contains("<player_message>{player_message}</player_message>"));
        assert!(config.contains("An English message requires an English answer"));
        assert!(config.contains("If there is no quoted player message, write only in English"));
        assert!(config.contains("OllamaChat.RandomChatterPromptTemplate = \"You are"));
        assert!(config.contains("World of Warcraft remark in English"));
        assert!(config.contains("OllamaChat.EventChatterPromptTemplate = \"You are"));
        assert!(config.contains("React only to this event in natural English"));
        assert!(config.contains("OllamaChat.Temperature = 0"));
        assert!(config.contains("OllamaChat.EnableChatHistory = 0"));
        assert!(config.contains("OllamaChat.ConversationHistorySaveInterval = 0"));
        assert!(config.contains("OllamaChat.EnableChatBotSnapshotTemplate = 0"));
        assert!(config.contains("OllamaChat.Memory.Enable = 0"));
        assert!(config.contains("OllamaChat.Relationship.Enable = 0"));
        assert!(config.contains("OllamaChat.EnableRAG = 0"));
        assert!(config.contains("OllamaChat.EnableEmoteReactions = 0"));
        let chat_prompt = config
            .lines()
            .find(|line| line.starts_with("OllamaChat.ChatPromptTemplate ="))
            .expect("chat prompt");
        assert!(!chat_prompt.contains("{chat_history}"));
        assert!(config.contains("OllamaChat.BotReplyChance.Channel = 0"));
        assert!(config.contains("OllamaChat.UnmanagedDefault = keep"));
        write_ollama_chat_config(
            temporary.path(),
            true,
            Some("qwen3:8b"),
            DialogueChattiness::Lively,
            DialogueLanguage::English,
        )
        .expect("lively dialogue");
        let lively = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/mod_ollama_chat.conf"),
        )
        .expect("lively config");
        assert!(lively.contains("OllamaChat.EnableRandomChatter = 1"));
        assert!(lively.contains("OllamaChat.EventChatterBotCommentChance = 10"));
        assert!(lively.contains("OllamaChat.EventChatterBotSelfCommentChance = 2"));
        assert!(lively.contains("OllamaChat.RandomChatterBotCommentChance = 35"));
        assert!(lively.contains("OllamaChat.BotReplyChance.Say = 35"));
        assert!(lively.contains("OllamaChat.BotReplyChance.Party = 100"));
        assert!(lively.contains("OllamaChat.Cooldown.PerBotSeconds = 60"));
        assert!(lively.contains("OllamaChat.Cooldown.PerScopeSeconds = 0"));
        assert!(lively.contains("OllamaChat.RateLimit.ScopePerMinute = 4"));
        assert!(lively.contains("OllamaChat.RateLimit.GlobalPerMinute = 6"));
        assert!(lively.contains("OllamaChat.MinRandomInterval = 30"));
        assert!(lively.contains("OllamaChat.MaxRandomInterval = 90"));
        assert!(lively.contains("OllamaChat.RandomChatterRealPlayerDistance = 150.0"));
        assert!(lively.contains("OllamaChat.BotConversation.MaxChainDepth = 2"));
        write_ollama_chat_config(
            temporary.path(),
            true,
            Some("qwen3:8b"),
            DialogueChattiness::Quiet,
            DialogueLanguage::English,
        )
        .expect("quiet dialogue");
        let quiet = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/mod_ollama_chat.conf"),
        )
        .expect("quiet config");
        assert!(quiet.contains("OllamaChat.EnableRandomChatter = 0"));
        assert!(quiet.contains("OllamaChat.EnableEventChatter = 0"));
        assert!(quiet.contains("OllamaChat.PlayerReplyChance.Say = 100"));
        assert!(quiet.contains("OllamaChat.PlayerReplyChance.Party = 100"));
        assert!(quiet.contains("OllamaChat.BotReplyChance.Say = 0"));
        assert!(quiet.contains("OllamaChat.BotReplyChance.Party = 0"));
        assert!(quiet.contains("OllamaChat.BotConversation.MaxChainDepth = 1"));

        write_ollama_chat_config(
            temporary.path(),
            true,
            Some("qwen3:8b"),
            DialogueChattiness::Balanced,
            DialogueLanguage::French,
        )
        .expect("French dialogue");
        let french = fs::read_to_string(
            temporary
                .path()
                .join("env/dist/etc/modules/mod_ollama_chat.conf"),
        )
        .expect("French config");
        assert!(french.contains("If there is no quoted player message, write only in French"));
        assert!(french.contains("une seule remarque naturelle et crédible en français"));
        assert!(french.contains("Réagis uniquement à cet événement"));
        assert!(french.contains("Reply directly to that message in the same language"));
        let environment = ollama_environment(Path::new("/managed/ai/models"), true);
        assert!(environment.contains(&(OsString::from("OLLAMA_NO_CLOUD"), OsString::from("true"))));
        assert!(environment.contains(&(OsString::from("OLLAMA_MAX_QUEUE"), OsString::from("8"))));
        assert!(
            write_ollama_chat_config(
                temporary.path(),
                true,
                Some("remote.example/model:latest"),
                DialogueChattiness::Balanced,
                DialogueLanguage::English,
            )
            .is_err()
        );
    }

    #[test]
    fn ollama_configuration_is_optional_when_the_module_was_not_installed() {
        let temporary = tempfile::tempdir().expect("tempdir");
        write_ollama_chat_config(
            temporary.path(),
            false,
            None,
            DialogueChattiness::Balanced,
            DialogueLanguage::English,
        )
        .expect("disabled module");
        assert!(!temporary.path().join("env/dist/etc/modules").exists());
    }

    #[test]
    fn dialogue_runtime_update_is_backed_up_staged_and_keeps_a_rollback() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let server_root = runtime_root.join("server");
        let compose_file = server_root.join("compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client dir");
        fs::create_dir_all(server_root.join("env/dist/etc/modules")).expect("module dir");
        fs::write(&client_executable, "binary").expect("client");
        fs::write(&compose_file, "services: {}\n").expect("compose");
        fs::write(server_root.join(".env"), "REALMBOX_DB_PASSWORD=test\n").expect("env");
        fs::write(server_root.join("keep-for-rollback"), "old server").expect("sentinel");
        fs::write(
            server_root.join("env/dist/etc/modules/playerbots.conf.dist"),
            playerbots_fixture(true, 5, ""),
        )
        .expect("playerbots config");
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");
        let mut record = InstallationRecord {
            schema_version: INSTALL_SCHEMA,
            game_data_root: temporary.path().join("game-source"),
            runtime_root: runtime_root.clone(),
            client_executable,
            client_choice: ClientChoice::ManagedOpenWow,
            compose_file,
            bots_enabled: true,
            bot_count: 50,
            ai_enabled: true,
            ai_model: Some("llama3.2:3b".into()),
            ollama_executable: None,
            client_sha256: Some("test-openwow-sha256".into()),
            ollama_sha256: None,
            server_commit: SERVER_COMMIT.into(),
            playerbots_commit: PLAYERBOTS_COMMIT.into(),
            ollama_chat_commit: None,
            runtime_release: Some("0.2.0-schema-3".into()),
        };
        service.save_record(&record).expect("record");
        let digest = "a".repeat(64);
        let images = ServerImages {
            authserver: format!("ghcr.io/example/auth@sha256:{digest}"),
            worldserver: format!("ghcr.io/example/world@sha256:{digest}"),
            db_import: format!("ghcr.io/example/db@sha256:{digest}"),
            tools: format!("ghcr.io/example/tools@sha256:{digest}"),
        };

        service
            .prepare_server_runtime_update(&mut record, &images, &mut |_| {})
            .expect("runtime update");

        assert!(server_root.join("compose.realmbox.yaml").is_file());
        assert!(
            server_root
                .join("env/dist/etc/modules/mod_ollama_chat.conf.dist")
                .is_file()
        );
        assert!(!server_root.join("keep-for-rollback").exists());
        let rollback_root = temporary.path().join(RUNTIME_ROLLBACK_DIRECTORY);
        let rollback = fs::read_dir(&rollback_root)
            .expect("rollback root")
            .next()
            .expect("rollback entry")
            .expect("rollback path")
            .path()
            .join("server/keep-for-rollback");
        assert_eq!(
            fs::read_to_string(rollback).expect("sentinel"),
            "old server"
        );
        assert!(
            fs::read_dir(temporary.path().join(PLAYER_DATA_BACKUP_DIRECTORY))
                .expect("backup root")
                .any(|entry| entry.expect("backup entry").path().extension()
                    == Some(OsStr::new("sql")))
        );
        assert_eq!(
            record.runtime_release,
            Some(pending_runtime_release_id(&runtime_release_id()))
        );
        assert!(
            fs::read_to_string(server_root.join("env/dist/etc/modules/mod_ollama_chat.conf"))
                .expect("dialogue config")
                .contains("OllamaChat.Enable = 1")
        );
        assert!(
            fs::read_to_string(server_root.join("env/dist/etc/modules/mod_ollama_chat.conf"))
                .expect("dialogue config")
                .contains("OllamaChat.Model = llama3.2:3b")
        );
        let commands = service.runner.commands.lock().expect("commands");
        let backup_index = commands
            .iter()
            .position(|command| command.contains("mysqldump"))
            .expect("backup command");
        let publish_index = commands
            .iter()
            .position(|command| command.contains("down --remove-orphans"))
            .expect("safe stop");
        assert!(backup_index < publish_index);
        assert!(commands.iter().any(|command| {
            command.contains("/azerothcore/env/ref/etc/modules/mod_ollama_chat.conf.dist")
        }));
        assert!(!commands.iter().any(|command| command.contains("--volumes")));
        drop(commands);

        service
            .prepare_server_runtime_update(&mut record, &images, &mut |_| {})
            .expect("prepared runtime retry");
        assert_eq!(
            service
                .runner
                .commands
                .lock()
                .expect("commands")
                .iter()
                .filter(|command| command.contains("mysqldump"))
                .count(),
            1
        );
    }

    #[test]
    fn interrupted_runtime_publication_resumes_without_a_second_backup() {
        for topology in ["before-rename", "after-first-rename", "after-publish"] {
            let temporary = tempfile::tempdir().expect("tempdir");
            let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
            let current_server = runtime_root.join("server");
            let client_executable = runtime_root.join("client/openwow-client");
            fs::create_dir_all(client_executable.parent().expect("client")).expect("client dir");
            fs::write(&client_executable, "binary").expect("client");
            let source_release = "0.3.4-schema-3";
            let target_release = runtime_release_id();
            let transition = migration_backup_stem(source_release, &target_release);
            let staging_root = temporary.path().join(format!(".{transition}-staging"));
            let staged_server = staging_root.join("server");
            let rollback_root = temporary
                .path()
                .join(RUNTIME_ROLLBACK_DIRECTORY)
                .join(&transition);
            let rollback_server = rollback_root.join("server");
            let digest = "a".repeat(64);
            let images = ServerImages {
                authserver: format!("ghcr.io/example/auth@sha256:{digest}"),
                worldserver: format!("ghcr.io/example/world@sha256:{digest}"),
                db_import: format!("ghcr.io/example/db@sha256:{digest}"),
                tools: format!("ghcr.io/example/tools@sha256:{digest}"),
            };
            let write_old_server = |root: &Path| {
                fs::create_dir_all(root).expect("old server dir");
                fs::write(
                    root.join("compose.realmbox.yaml"),
                    "services: {}\n# old-runtime\n",
                )
                .expect("old compose");
            };
            let write_new_server = |root: &Path| {
                fs::create_dir_all(root).expect("new server dir");
                fs::write(
                    root.join("compose.realmbox.yaml"),
                    compose_file(
                        "1000",
                        "1000",
                        temporary.path().join("game-source").as_path(),
                        DEFAULT_DOCKER_BUILD_JOBS,
                        Some(&images),
                    ),
                )
                .expect("new compose");
            };
            match topology {
                "before-rename" => {
                    write_old_server(&current_server);
                    write_new_server(&staged_server);
                }
                "after-first-rename" => {
                    write_old_server(&rollback_server);
                    write_new_server(&staged_server);
                }
                "after-publish" => {
                    write_old_server(&rollback_server);
                    write_new_server(&current_server);
                }
                _ => unreachable!(),
            }

            let backup_root = temporary.path().join(PLAYER_DATA_BACKUP_DIRECTORY);
            fs::create_dir_all(&backup_root).expect("backup root");
            let backup = backup_root.join(format!("{transition}.sql"));
            fs::write(
                &backup,
                "-- Current Database: `acore_auth`\n-- Current Database: `acore_characters`\n-- Current Database: `acore_playerbots`\n-- Current Database: `acore_world`\n",
            )
            .expect("backup");
            fs::write(
                backup.with_extension("sha256"),
                format!("{}\n", sha256_file(&backup).expect("checksum")),
            )
            .expect("checksum file");
            let recovery = RecoveryMetadata {
                schema_version: 1,
                stem: transition.clone(),
                source_runtime_release: Some(source_release.into()),
                target_runtime_release: target_release.clone(),
                ai_enabled: false,
                ai_model: None,
                ollama_chat_commit: None,
            };
            let transaction = RuntimeUpdateTransaction {
                schema_version: 1,
                transition: transition.clone(),
                attempt: 1,
                phase: RuntimeUpdatePhase::Staged,
                images: images.clone(),
                recovery,
            };
            fs::write(
                temporary.path().join(RUNTIME_UPDATE_FILE),
                serde_json::to_vec_pretty(&transaction).expect("transaction"),
            )
            .expect("transaction marker");

            let mut service = LauncherService::new(
                temporary.path().to_path_buf(),
                temporary.path().join("addon"),
                RecordingRunner::default(),
            )
            .expect("service");
            let mut record = InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: temporary.path().join("game-source"),
                runtime_root: runtime_root.clone(),
                client_executable: client_executable.clone(),
                client_choice: ClientChoice::ManagedOpenWow,
                compose_file: current_server.join("compose.realmbox.yaml"),
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
                runtime_release: Some(source_release.into()),
            };
            service.save_record(&record).expect("record");

            if topology == "after-first-rename" {
                let status = service.bootstrap(|_| {}).expect("public bootstrap resume");
                assert_eq!(status.phase, LauncherPhase::Ready);
                record = service.load_record().expect("record").expect("installed");
            } else {
                assert!(
                    service
                        .resume_runtime_update_if_needed(&mut record)
                        .expect("resume")
                );
            }
            assert!(runtime_server_matches_images(&current_server, &images).expect("images"));
            assert!(rollback_server.join("compose.realmbox.yaml").is_file());
            assert_eq!(
                record.runtime_release,
                Some(pending_runtime_release_id(&target_release))
            );
            validate_prepared_runtime_update(temporary.path(), &record, &images, &target_release)
                .expect("prepared runtime validation");
            assert!(!temporary.path().join(RUNTIME_UPDATE_FILE).exists());
            assert_eq!(
                service
                    .runner
                    .commands
                    .lock()
                    .expect("commands")
                    .iter()
                    .filter(|command| command.contains("mysqldump"))
                    .count(),
                0,
                "{topology}"
            );
            assert!(
                service
                    .runner
                    .commands
                    .lock()
                    .expect("commands")
                    .iter()
                    .all(|command| !command.contains(" up "))
            );
        }
    }

    #[test]
    fn forged_runtime_transaction_stem_is_rejected_before_path_construction() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let current_server = runtime_root.join("server");
        fs::create_dir_all(&current_server).expect("server");
        fs::write(
            current_server.join("compose.realmbox.yaml"),
            "services: {}\n",
        )
        .expect("compose");
        let digest = "a".repeat(64);
        let images = ServerImages {
            authserver: format!("ghcr.io/example/auth@sha256:{digest}"),
            worldserver: format!("ghcr.io/example/world@sha256:{digest}"),
            db_import: format!("ghcr.io/example/db@sha256:{digest}"),
            tools: format!("ghcr.io/example/tools@sha256:{digest}"),
        };
        let forged = "../../outside";
        let recovery = RecoveryMetadata {
            schema_version: 1,
            stem: forged.into(),
            source_runtime_release: Some("0.3.4-schema-3".into()),
            target_runtime_release: runtime_release_id(),
            ai_enabled: false,
            ai_model: None,
            ollama_chat_commit: None,
        };
        fs::write(
            temporary.path().join(RUNTIME_UPDATE_FILE),
            serde_json::to_vec_pretty(&RuntimeUpdateTransaction {
                schema_version: 1,
                transition: forged.into(),
                attempt: 1,
                phase: RuntimeUpdatePhase::Staged,
                images,
                recovery,
            })
            .expect("marker"),
        )
        .expect("marker file");
        let mut record = InstallationRecord {
            schema_version: INSTALL_SCHEMA,
            game_data_root: temporary.path().join("game-source"),
            runtime_root,
            client_executable: temporary.path().join("client"),
            client_choice: ClientChoice::ManagedOpenWow,
            compose_file: current_server.join("compose.realmbox.yaml"),
            bots_enabled: true,
            bot_count: 50,
            ai_enabled: false,
            ai_model: None,
            ollama_executable: None,
            client_sha256: None,
            ollama_sha256: None,
            server_commit: SERVER_COMMIT.into(),
            playerbots_commit: PLAYERBOTS_COMMIT.into(),
            ollama_chat_commit: None,
            runtime_release: Some("0.3.4-schema-3".into()),
        };
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");

        let error = service
            .resume_runtime_update_if_needed(&mut record)
            .expect_err("forged transaction must fail");
        assert!(error.contains("incohérent"));
        assert!(
            !temporary
                .path()
                .parent()
                .expect("parent")
                .join("outside")
                .exists()
        );
    }

    #[test]
    fn bootstrap_reports_ready_without_starting_an_installed_world() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::write(&compose_file, "services: {}\n").expect("compose");
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
                ai_enabled: false,
                ai_model: None,
                ollama_executable: None,
                client_sha256: Some("test-openwow-sha256".into()),
                ollama_sha256: None,
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: None,
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");

        let status = service.bootstrap(|_| {}).expect("bootstrap");
        assert_eq!(status.phase, LauncherPhase::Ready);
        let commands = service.runner.commands.lock().expect("commands");
        assert!(
            commands
                .iter()
                .any(|command| command.contains("ps --status running"))
        );
        assert!(!commands.iter().any(|command| command.contains(" up ")));
        assert_eq!(service.client_process_id(), None);
        drop(commands);

        *service
            .runner
            .docker_volumes_present
            .lock()
            .expect("docker volumes") = Some(false);
        let purged = service.bootstrap(|_| {}).expect("purged bootstrap");
        assert_eq!(purged.phase, LauncherPhase::Ready);
        assert!(purged.message.contains("seront reconstruites"));
    }

    #[test]
    fn ollama_model_manifest_must_match_the_pinned_digest() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let manifest = temporary
            .path()
            .join("manifests/registry.ollama.ai/library/llama3.2/3b");
        fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("manifest dir");
        fs::write(&manifest, "mutable tag content").expect("manifest");

        let error = verify_ollama_model_manifest(temporary.path(), "llama3.2:3b")
            .expect_err("mutable manifest must be rejected");
        assert!(error.contains("modèle épinglé"));
    }

    #[test]
    fn start_refuses_a_second_owned_client_before_any_runtime_or_docker_action() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
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
                client_executable: runtime_root.join("client/openwow-client"),
                client_choice: ClientChoice::ManagedOpenWow,
                compose_file: runtime_root.join("server/compose.realmbox.yaml"),
                bots_enabled: true,
                bot_count: 50,
                ai_enabled: false,
                ai_model: None,
                ollama_executable: None,
                client_sha256: None,
                ollama_sha256: None,
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: None,
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");
        service.client_process_id = Some(42);

        let error = service
            .start(None, None, None, None, |_| {})
            .expect_err("second start must fail");
        assert!(error.contains("déjà lancé"));
        assert_eq!(service.client_process_id(), Some(42));
        assert!(service.runner.commands.lock().expect("commands").is_empty());
    }

    #[test]
    fn installed_dialogue_can_be_enabled_and_disabled_without_redownloading() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        let ollama_executable = runtime_root.join("ai/ollama");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::create_dir_all(ollama_executable.parent().expect("ai")).expect("ai");
        fs::write(&compose_file, "services: {}").expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        fs::write(&ollama_executable, "binary").expect("ollama");
        write_ollama_chat_fixture(
            &runtime_root.join("server/env/dist/etc/modules/mod_ollama_chat.conf.dist"),
        );
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
                bot_count: 50,
                ai_enabled: false,
                ai_model: Some("llama3.2:3b".into()),
                ollama_executable: Some(ollama_executable),
                client_sha256: Some("test-openwow-sha256".into()),
                ollama_sha256: Some("test-ollama-sha256".into()),
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: Some(OLLAMA_CHAT_COMMIT.into()),
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");
        let saved = service.load_record().expect("record").expect("installed");
        assert!(!local_dialogue_download_required(&saved, "llama3.2:3b"));
        assert!(local_dialogue_download_required(&saved, "qwen2.5:3b"));

        let enabled = service
            .configure_local_dialogue(true, Some("llama3.2:3b".into()), |_| {})
            .expect("enable");
        assert!(enabled.ai_enabled);
        assert_eq!(enabled.ai_model.as_deref(), Some("llama3.2:3b"));
        assert!(
            !service
                .runner
                .commands
                .lock()
                .expect("commands")
                .iter()
                .any(|command| command.starts_with("curl "))
        );

        let disabled = service
            .configure_local_dialogue(false, None, |_| {})
            .expect("disable");
        assert!(!disabled.ai_enabled);
        assert_eq!(disabled.ai_model.as_deref(), Some("llama3.2:3b"));
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
    fn pre_migration_backup_is_verified_checksummed_and_never_overwritten() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::default();
        let compose = temporary
            .path()
            .join("runtime/server/compose.realmbox.yaml");
        fs::create_dir_all(compose.parent().expect("server")).expect("server");
        fs::write(&compose, "services: {}").expect("compose");

        let first = create_pre_migration_backup(
            &runner,
            temporary.path(),
            &compose,
            compose.parent().expect("server"),
            "0.1.0-schema-2",
            &runtime_release_id(),
            &temporary.path().join("logs/backup.log"),
        )
        .expect("first backup");
        let first_contents = fs::read(&first).expect("backup contents");
        let second = create_pre_migration_backup(
            &runner,
            temporary.path(),
            &compose,
            compose.parent().expect("server"),
            "0.1.0-schema-2",
            &runtime_release_id(),
            &temporary.path().join("logs/backup.log"),
        )
        .expect("reused backup");

        assert_eq!(first, second);
        assert_eq!(fs::read(&second).expect("preserved backup"), first_contents);
        assert!(first.starts_with(temporary.path().join(PLAYER_DATA_BACKUP_DIRECTORY)));
        assert!(first.with_extension("sha256").is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&first)
                .expect("backup metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert_eq!(
            runner
                .commands
                .lock()
                .expect("commands")
                .iter()
                .filter(|command| command.contains("mysqldump"))
                .count(),
            1
        );
    }

    #[test]
    fn manual_backup_is_complete_verified_unique_and_stops_only_its_database() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");
        save_test_installation(&service, temporary.path());

        let first = service.create_realm_backup().expect("first backup");
        let second = service.create_realm_backup().expect("second backup");
        assert!(first.size_bytes > 0);
        assert!(second.size_bytes > 0);
        assert!(
            service
                .inspect_realm_backup()
                .expect("backup inspection")
                .is_some()
        );

        let backup_root = temporary.path().join(PLAYER_DATA_BACKUP_DIRECTORY);
        let backups = fs::read_dir(&backup_root)
            .expect("backup root")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension() == Some(OsStr::new("sql")))
            .count();
        assert_eq!(backups, 2);
        let commands = service.runner.commands.lock().expect("commands");
        assert_eq!(
            commands
                .iter()
                .filter(|command| command.contains(" up -d --wait --wait-timeout 120 database"))
                .count(),
            2
        );
        assert_eq!(
            commands
                .iter()
                .filter(|command| command.contains(" stop database"))
                .count(),
            2
        );
        assert_eq!(
            commands
                .iter()
                .filter(|command| command.contains("mysqldump"))
                .count(),
            2
        );
        assert!(
            !commands
                .iter()
                .any(|command| command.contains("--volumes") || command.contains(" -v "))
        );
    }

    #[test]
    fn manual_backup_keeps_an_already_running_database_online() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner {
            running_services: Mutex::new(vec!["database".into()]),
            ..Default::default()
        };
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            runner,
        )
        .expect("service");
        save_test_installation(&service, temporary.path());

        service.create_realm_backup().expect("running backup");

        let commands = service.runner.commands.lock().expect("commands");
        assert!(commands.iter().any(|command| command.contains("mysqldump")));
        assert!(
            !commands
                .iter()
                .any(|command| command.contains(" up -d --wait")
                    || command.contains(" stop database"))
        );
    }

    #[test]
    fn manual_backup_refuses_a_missing_player_volume_before_starting_mysql() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner {
            docker_volumes_present: Mutex::new(Some(false)),
            ..Default::default()
        };
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            runner,
        )
        .expect("service");
        save_test_installation(&service, temporary.path());

        let error = service
            .create_realm_backup()
            .expect_err("a missing volume must never become an empty backup target");
        assert!(error.contains("volume Docker des personnages a disparu"));
        let commands = service.runner.commands.lock().expect("commands");
        assert!(
            commands
                .iter()
                .any(|command| command.contains("volume inspect"))
        );
        assert!(!commands.iter().any(|command| {
            command.contains(" up -d --wait")
                || command.contains("mysqldump")
                || command.contains(" stop database")
        }));
    }

    #[test]
    fn docker_purge_selects_a_verified_manual_backup_and_resumes_from_its_marker() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let backup_root = temporary.path().join(PLAYER_DATA_BACKUP_DIRECTORY);
        fs::create_dir_all(&backup_root).expect("backup root");
        let backup = backup_root.join("manual-backup-dockerpurge.sql");
        fs::write(
            &backup,
            "-- Current Database: `acore_auth`\n-- Current Database: `acore_characters`\n-- Current Database: `acore_playerbots`\n-- Current Database: `acore_world`\n",
        )
        .expect("backup");
        fs::write(
            backup.with_extension("sha256"),
            format!("{}\n", sha256_file(&backup).expect("checksum")),
        )
        .expect("checksum");

        let selected = prepare_docker_recovery(temporary.path(), true)
            .expect("purge recovery")
            .expect("selected backup");
        assert_eq!(selected.stem, "manual-backup-dockerpurge");
        assert!(temporary.path().join(DOCKER_RECOVERY_FILE).is_file());

        let resumed = prepare_docker_recovery(temporary.path(), false)
            .expect("resume recovery")
            .expect("marked backup");
        assert_eq!(resumed, selected);
    }

    #[test]
    fn docker_purge_without_a_verified_backup_fails_closed() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let error = prepare_docker_recovery(temporary.path(), true)
            .expect_err("missing player backup must block an empty realm");
        assert!(error.contains("volume Docker des personnages a disparu"));
        assert!(error.contains("ne pas créer un royaume vide"));
        assert!(!temporary.path().join(DOCKER_RECOVERY_FILE).exists());
    }

    #[test]
    fn verified_recovery_restores_runtime_and_database_without_deleting_volumes() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let current_server = runtime_root.join("server");
        let compose_file = current_server.join("compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        fs::create_dir_all(&current_server).expect("current server");
        fs::create_dir_all(client_executable.parent().expect("client parent")).expect("client");
        fs::write(&compose_file, "services: {}\n# current-runtime\n").expect("compose");
        fs::write(&client_executable, "client").expect("client");

        let stem = "pre-migration-recoverytest";
        let rollback_root = temporary.path().join(RUNTIME_ROLLBACK_DIRECTORY).join(stem);
        let rollback_server = rollback_root.join("server");
        fs::create_dir_all(&rollback_server).expect("rollback server");
        fs::write(
            rollback_server.join("compose.realmbox.yaml"),
            "services: {}\n# last-working-runtime\n",
        )
        .expect("rollback compose");
        let metadata = RecoveryMetadata {
            schema_version: 1,
            stem: stem.into(),
            source_runtime_release: Some("0.2.3-schema-3".into()),
            target_runtime_release: runtime_release_id(),
            ai_enabled: false,
            ai_model: None,
            ollama_chat_commit: None,
        };
        fs::write(
            rollback_root.join("recovery.json"),
            serde_json::to_vec_pretty(&metadata).expect("metadata"),
        )
        .expect("metadata file");
        let backup_root = temporary.path().join(PLAYER_DATA_BACKUP_DIRECTORY);
        fs::create_dir_all(&backup_root).expect("backup root");
        let backup = backup_root.join(format!("{stem}.sql"));
        fs::write(
            &backup,
            "-- Current Database: `acore_auth`\n-- Current Database: `acore_characters`\n-- Current Database: `acore_playerbots`\n-- Current Database: `acore_world`\n",
        )
        .expect("backup");
        fs::write(
            backup_root.join(format!("{stem}.sha256")),
            format!("{}\n", sha256_file(&backup).expect("checksum")),
        )
        .expect("checksum file");

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
                compose_file: compose_file.clone(),
                bots_enabled: true,
                bot_count: 50,
                ai_enabled: true,
                ai_model: Some("llama3.2:3b".into()),
                ollama_executable: Some(temporary.path().join("ai/ollama")),
                client_sha256: Some("test-openwow-sha256".into()),
                ollama_sha256: Some("test-ollama-sha256".into()),
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: Some(OLLAMA_CHAT_COMMIT.into()),
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");

        assert!(service.status().recovery_available);
        let status = service
            .restore_last_recovery(|_| {})
            .expect("verified recovery");

        assert_eq!(status.phase, LauncherPhase::Ready);
        assert!(!status.recovery_available);
        assert!(
            fs::read_to_string(&compose_file)
                .expect("restored compose")
                .contains("last-working-runtime")
        );
        assert!(
            fs::read_to_string(
                temporary
                    .path()
                    .join(RUNTIME_ROLLBACK_DIRECTORY)
                    .join(format!("failed-after-{stem}/server/compose.realmbox.yaml")),
            )
            .expect("preserved failed runtime")
            .contains("current-runtime")
        );
        let restored_record = service.load_record().expect("record").expect("installed");
        assert_eq!(
            restored_record.runtime_release.as_deref(),
            Some("0.2.3-schema-3")
        );
        assert!(!restored_record.ai_enabled);
        let commands = service.runner.commands.lock().expect("commands");
        assert!(
            commands
                .iter()
                .any(|command| command.contains("exec mysql"))
        );
        assert!(
            commands
                .iter()
                .all(|command| !command.contains("--volumes"))
        );
    }

    #[test]
    fn failed_recovery_restores_the_original_runtime_and_safety_backup() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let current_server = runtime_root.join("server");
        let compose_file = current_server.join("compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        fs::create_dir_all(&current_server).expect("current server");
        fs::create_dir_all(client_executable.parent().expect("client parent")).expect("client");
        fs::write(&compose_file, "services: {}\n# current-runtime\n").expect("compose");
        fs::write(&client_executable, "client").expect("client");

        let stem = "pre-migration-recoveryfailure";
        let rollback_root = temporary.path().join(RUNTIME_ROLLBACK_DIRECTORY).join(stem);
        let rollback_server = rollback_root.join("server");
        fs::create_dir_all(&rollback_server).expect("rollback server");
        fs::write(
            rollback_server.join("compose.realmbox.yaml"),
            "services: {}\n# last-working-runtime\n",
        )
        .expect("rollback compose");
        let metadata = RecoveryMetadata {
            schema_version: 1,
            stem: stem.into(),
            source_runtime_release: Some("0.2.3-schema-3".into()),
            target_runtime_release: runtime_release_id(),
            ai_enabled: false,
            ai_model: None,
            ollama_chat_commit: None,
        };
        fs::write(
            rollback_root.join("recovery.json"),
            serde_json::to_vec_pretty(&metadata).expect("metadata"),
        )
        .expect("metadata file");
        let backup_root = temporary.path().join(PLAYER_DATA_BACKUP_DIRECTORY);
        fs::create_dir_all(&backup_root).expect("backup root");
        let backup = backup_root.join(format!("{stem}.sql"));
        fs::write(
            &backup,
            "-- Current Database: `acore_auth`\n-- Current Database: `acore_characters`\n-- Current Database: `acore_playerbots`\n-- Current Database: `acore_world`\n",
        )
        .expect("backup");
        fs::write(
            backup_root.join(format!("{stem}.sha256")),
            format!("{}\n", sha256_file(&backup).expect("checksum")),
        )
        .expect("checksum file");

        let runner = RecordingRunner {
            fail_next_input: Mutex::new(true),
            ..Default::default()
        };
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            runner,
        )
        .expect("service");
        service
            .save_record(&InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: temporary.path().join("game-source"),
                runtime_root: runtime_root.clone(),
                client_executable,
                client_choice: ClientChoice::ManagedOpenWow,
                compose_file: compose_file.clone(),
                bots_enabled: true,
                bot_count: 50,
                ai_enabled: true,
                ai_model: Some("llama3.2:3b".into()),
                ollama_executable: Some(temporary.path().join("ai/ollama")),
                client_sha256: Some("test-openwow-sha256".into()),
                ollama_sha256: Some("test-ollama-sha256".into()),
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: Some(OLLAMA_CHAT_COMMIT.into()),
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");

        let error = service
            .restore_last_recovery(|_| {})
            .expect_err("first database import must fail");

        assert!(error.contains("état précédent rétabli"));
        assert!(
            fs::read_to_string(&compose_file)
                .expect("original compose restored")
                .contains("current-runtime")
        );
        assert!(
            fs::read_to_string(rollback_server.join("compose.realmbox.yaml"))
                .expect("recovery point preserved")
                .contains("last-working-runtime")
        );
        assert!(
            !temporary
                .path()
                .join(RUNTIME_ROLLBACK_DIRECTORY)
                .join(format!("failed-after-{stem}"))
                .exists()
        );
        let record = service.load_record().expect("record").expect("installed");
        assert_eq!(record.runtime_release, Some(runtime_release_id()));
        assert!(record.ai_enabled);
        let commands = service.runner.commands.lock().expect("commands");
        assert_eq!(
            commands
                .iter()
                .filter(|command| command.contains("exec mysql") && command.contains(" < "))
                .count(),
            2
        );
        assert!(
            commands
                .iter()
                .all(|command| !command.contains("--volumes"))
        );
    }

    #[test]
    fn incomplete_database_backup_blocks_migration() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let backup = temporary.path().join("backup.sql");
        fs::write(&backup, "-- Current Database: `acore_auth`\n").expect("partial backup");
        let error = validate_database_backup(&backup).expect_err("incomplete dump");
        assert!(error.contains("migration annulée"));
    }

    #[test]
    fn fresh_install_never_replaces_an_existing_realm() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime = temporary.path().join(RUNTIME_DIRECTORY);
        fs::create_dir_all(&runtime).expect("existing runtime");
        fs::write(runtime.join("character-sentinel"), "keep").expect("sentinel");
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");

        let error = service
            .ensure_fresh_install_target()
            .expect_err("existing realm must be protected");
        assert!(error.contains("réinstalle jamais"));
        assert_eq!(
            fs::read_to_string(runtime.join("character-sentinel")).expect("sentinel preserved"),
            "keep"
        );
    }

    #[test]
    fn setup_check_preserves_existing_realm_and_rejects_unknown_model() {
        let temporary = tempfile::tempdir().expect("tempdir");
        fs::write(
            temporary.path().join("installation.json"),
            "unknown schema sentinel",
        )
        .unwrap();
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .unwrap();
        assert!(!service.inspect_installation(None).unwrap().fresh_target);
        assert!(
            service
                .inspect_installation(Some("untrusted:latest"))
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(temporary.path().join("installation.json")).unwrap(),
            "unknown schema sentinel"
        );
        assert!(!temporary.path().join(".installing-v3").exists());
    }

    #[cfg(unix)]
    #[test]
    fn dangling_installation_link_is_not_a_fresh_realm() {
        let temporary = tempfile::tempdir().expect("tempdir");
        std::os::unix::fs::symlink(
            temporary.path().join("missing"),
            temporary.path().join("installation.json"),
        )
        .unwrap();
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .unwrap();
        assert!(service.ensure_fresh_install_target().is_err());
        assert!(!service.inspect_installation(None).unwrap().fresh_target);
    }

    #[test]
    fn unknown_installation_schema_is_an_error_not_an_empty_realm() {
        let temporary = tempfile::tempdir().expect("tempdir");
        fs::write(
            temporary.path().join("installation.json"),
            r#"{"schemaVersion":999}"#,
        )
        .expect("future manifest");
        let service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner::default(),
        )
        .expect("service");

        let error = service.load_record().expect_err("unknown schema");
        assert!(error.contains("les données sont conservées"));
        assert_eq!(service.status().phase, LauncherPhase::Error);
    }

    #[test]
    #[should_panic(expected = "suppression de volumes persistants")]
    fn compose_volume_deletion_is_rejected_before_execution() {
        let _ = compose_args(
            Path::new("/managed/server/compose.realmbox.yaml"),
            &["down", "--volumes"],
        );
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
                runtime_release: Some(runtime_release_id()),
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
        let runner = SystemCommandRunner::default();
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
    fn integrated_downloader_resumes_and_publishes_only_after_checksum() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let destination = temporary.path().join("client.zip");
        let partial = temporary.path().join("client.zip.part");
        fs::write(&partial, b"hello ").expect("partial");
        let payload = b"hello world";
        let expected = format!("{:x}", Sha256::digest(payload));
        let mut observed = Vec::new();
        let mut remainder = std::io::Cursor::new(b"world");

        let downloaded = append_download_body(
            &mut remainder,
            &partial,
            true,
            6,
            Some(11),
            &mut |completed, total| observed.push((completed, total)),
        )
        .expect("resume body");
        publish_verified_download(&partial, &destination, &expected).expect("verified publication");

        assert_eq!(downloaded, 11);
        assert_eq!(fs::read(&destination).expect("published"), payload);
        assert!(!partial.exists());
        assert_eq!(observed.last(), Some(&(11, Some(11))));
    }

    #[test]
    fn integrated_downloader_removes_a_checksum_mismatch() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let destination = temporary.path().join("client.zip");
        let partial = temporary.path().join("client.zip.part");
        fs::write(&partial, b"bad").expect("partial");

        let error = publish_verified_download(&partial, &destination, &"0".repeat(64))
            .expect_err("checksum mismatch");

        assert!(error.contains("SHA-256"));
        assert!(!destination.exists());
        assert!(!partial.exists());
    }

    #[cfg(unix)]
    #[test]
    fn system_runner_refuses_to_terminate_an_unowned_process() {
        let runner = SystemCommandRunner::default();
        let error = runner
            .terminate(std::process::id())
            .expect_err("current test process is not owned by the runner");
        assert!(error.contains("refuse d’arrêter"));
    }

    #[cfg(unix)]
    #[test]
    fn system_runner_terminates_its_owned_process_group() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = SystemCommandRunner::default();
        let process_id = runner
            .spawn(
                Path::new("/bin/sh"),
                &["-c".into(), "sleep 30".into()],
                &[],
                None,
                &temporary.path().join("client.log"),
            )
            .expect("spawn owned process group");
        assert!(
            runner
                .is_process_running(process_id)
                .expect("inspect owned")
        );

        runner.terminate(process_id).expect("terminate owned group");

        assert!(
            !runner
                .is_process_running(process_id)
                .expect("ownership is cleared after termination")
        );
    }

    #[cfg(unix)]
    #[test]
    fn system_runner_bounded_commands_kill_owned_groups_at_the_deadline() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runner = SystemCommandRunner::default();
        let args = ["-c".into(), "trap '' TERM; sleep 30".into()];
        let started = Instant::now();
        let error = runner
            .run_bounded("/bin/sh", &args, None, Duration::from_millis(50))
            .expect_err("deadline");
        assert!(error.contains("dépassé le délai"));
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(runner.owned_processes.lock().expect("owned").is_empty());

        let started = Instant::now();
        let error = runner
            .run_long_bounded(
                "/bin/sh",
                &args,
                None,
                &temporary.path().join("bounded.log"),
                Duration::from_millis(50),
            )
            .expect_err("logged deadline");
        assert!(error.contains("dépassé le délai"));
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(runner.owned_processes.lock().expect("owned").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn system_runner_bounded_capture_rejects_large_output_without_pipe_deadlock() {
        let runner = SystemCommandRunner::default();
        let started = Instant::now();
        let error = runner
            .run_bounded(
                "/bin/dd",
                &["if=/dev/zero".into(), "bs=1024".into(), "count=128".into()],
                None,
                Duration::from_secs(2),
            )
            .expect_err("output cap");
        assert!(error.contains("64 KiB"));
        assert!(!error.contains("dépassé le délai"));
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(runner.owned_processes.lock().expect("owned").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn system_runner_bounded_capture_closes_descendants_inheriting_output() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let marker = temporary.path().join("orphan-marker");
        let runner = SystemCommandRunner::default();
        let started = Instant::now();
        let output = runner
            .run_bounded(
                "/bin/sh",
                &[
                    "-c".into(),
                    "(sleep 0.2; printf late > \"$1\") & printf ready".into(),
                    "realmbox-bounded-test".into(),
                    marker.as_os_str().into(),
                ],
                None,
                Duration::from_secs(2),
            )
            .expect("completed parent");
        assert_eq!(output, "ready");
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(runner.owned_processes.lock().expect("owned").is_empty());
        thread::sleep(Duration::from_millis(300));
        assert!(
            !marker.exists(),
            "owned descendant must not survive completion"
        );
    }

    #[cfg(unix)]
    #[test]
    fn bounded_capture_is_private_and_removed_on_drop() {
        use std::os::unix::fs::PermissionsExt;

        let capture = BoundedCommandCapture::new().expect("capture");
        let path = capture.path.clone();
        assert_eq!(
            fs::metadata(&path).expect("metadata").permissions().mode() & 0o777,
            0o600
        );
        drop(capture);
        assert!(!path.exists());
    }

    #[test]
    fn legacy_start_restores_a_missing_database_then_fails_closed_without_release_images() {
        // The release workflow deliberately compiles the launcher with all four images.
        // The no-image path remains covered by the normal validation build.
        if embedded_server_images()
            .expect("compile-time image set is valid")
            .is_some()
        {
            return;
        }
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        let addon = temporary.path().join("addon");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::create_dir_all(&addon).expect("addon");
        fs::write(&compose_file, "services: {}\n# legacy-runtime\n").expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        for filename in [
            "RealmBoxCompanions.lua",
            "RealmBoxCompanions.toc",
            "RealmBoxCompanions.xml",
        ] {
            fs::write(addon.join(filename), format!("fixture {filename}")).expect("addon fixture");
        }
        let backup_root = temporary.path().join(PLAYER_DATA_BACKUP_DIRECTORY);
        fs::create_dir_all(&backup_root).expect("backup root");
        let backup = backup_root.join("pre-migration-before-legacy-update.sql");
        fs::write(
            &backup,
            "-- Current Database: `acore_auth`\n-- Current Database: `acore_characters`\n-- Current Database: `acore_playerbots`\n-- Current Database: `acore_world`\n",
        )
        .expect("backup");
        fs::write(
            backup.with_extension("sha256"),
            format!("{}\n", sha256_file(&backup).expect("checksum")),
        )
        .expect("checksum");
        let runner = RecordingRunner {
            docker_volumes_present: Mutex::new(Some(false)),
            ..Default::default()
        };
        let mut service =
            LauncherService::new(temporary.path().to_path_buf(), addon, runner).expect("service");
        let source_release = "0.3.4-schema-3";
        service
            .save_record(&InstallationRecord {
                schema_version: INSTALL_SCHEMA,
                game_data_root: temporary.path().join("game-source"),
                runtime_root: runtime_root.clone(),
                client_executable,
                client_choice: ClientChoice::ManagedOpenWow,
                compose_file,
                bots_enabled: true,
                bot_count: 50,
                ai_enabled: false,
                ai_model: None,
                ollama_executable: None,
                client_sha256: None,
                ollama_sha256: None,
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: None,
                runtime_release: Some(source_release.into()),
            })
            .expect("record");

        let error = service
            .start(None, None, None, None, |_| {})
            .expect_err("development build must not promote a legacy runtime");
        assert!(error.contains("images serveur immuables"));
        assert_eq!(
            service
                .load_record()
                .expect("record")
                .expect("installed")
                .runtime_release
                .as_deref(),
            Some(source_release)
        );
        let commands = service.runner.commands.lock().expect("commands");
        let restore_index = commands
            .iter()
            .position(|command| {
                command.contains("exec mysql")
                    && command.contains("pre-migration-before-legacy-update.sql")
            })
            .expect("database restore");
        assert!(
            commands[..restore_index]
                .iter()
                .any(|command| command.contains("up -d --wait --wait-timeout 120 database"))
        );
        assert!(
            !commands
                .iter()
                .any(|command| command.contains("run --rm db-import"))
        );
        assert!(!commands.iter().any(|command| command.contains("--volumes")));
    }

    #[test]
    fn managed_openwow_starts_from_its_writable_game_root() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        let managed_game = runtime_root.join("game");
        let addon = temporary.path().join("addon");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::create_dir_all(&managed_game).expect("managed game");
        fs::create_dir_all(&addon).expect("addon");
        for filename in [
            "RealmBoxCompanions.lua",
            "RealmBoxCompanions.toc",
            "RealmBoxCompanions.xml",
        ] {
            fs::write(addon.join(filename), format!("fresh {filename}")).expect("addon fixture");
        }
        let stale_addon = managed_game.join("Interface/AddOns/RealmBoxCompanions");
        fs::create_dir_all(&stale_addon).expect("stale addon dir");
        fs::write(stale_addon.join("RealmBoxCompanions.lua"), "stale").expect("stale addon");
        let playerbots_config = runtime_root.join("server/env/dist/etc/modules/playerbots.conf");
        fs::create_dir_all(playerbots_config.parent().expect("module config"))
            .expect("module config");
        fs::write(
            &compose_file,
            "services:\n  server-data-init:\n    volumes:\n      - realmbox-server-data:/work\n    command:\n      - >-\n          /azerothcore/env/dist/bin/vmap4_extractor;\n          mkdir -p /work/vmaps;\n          /azerothcore/env/dist/bin/vmap4_assembler /work/Buildings /work/vmaps;\n          /azerothcore/env/dist/bin/mmaps_generator --config /azerothcore/env/dist/bin/mmaps-config.yaml --silent;\n  worldserver:\n    environment:\n      TEST: value\n",
        )
        .expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        fs::write(playerbots_config, playerbots_fixture(false, 0, "")).expect("playerbots config");
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            addon,
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
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");

        let status = service
            .start(None, None, None, None, |_| {})
            .expect("start");
        assert_eq!(status.phase, LauncherPhase::Running);
        assert_eq!(service.client_process_id(), Some(42));
        assert_eq!(
            fs::read_to_string(stale_addon.join("RealmBoxCompanions.lua"))
                .expect("refreshed addon"),
            "fresh RealmBoxCompanions.lua"
        );
        let commands = service.runner.commands.lock().expect("commands");
        let migration_index = commands
            .iter()
            .position(|command| command.contains("run --rm db-import"))
            .expect("database migration");
        let auth_ready_index = commands
            .iter()
            .position(|command| command == "wait-service authserver:3724")
            .expect("authserver readiness");
        let world_ready_index = commands
            .iter()
            .position(|command| command == "wait-service worldserver:8085")
            .expect("worldserver readiness");
        let client_index = commands
            .iter()
            .position(|command| command.starts_with(&client_executable.display().to_string()))
            .expect("client launch");
        assert!(!commands.iter().any(|command| command.contains("mysqldump")));
        assert!(migration_index < auth_ready_index);
        assert!(auth_ready_index < world_ready_index);
        assert!(world_ready_index < client_index);
        assert!(commands.iter().any(|command| {
            command.starts_with(&client_executable.display().to_string())
                && command.contains(&format!("--game-data {}", managed_game.display()))
                && command.contains(&format!("cwd={}", managed_game.display()))
        }));
        drop(commands);
        assert_eq!(
            service
                .load_record()
                .expect("record")
                .expect("installed")
                .runtime_release,
            Some(runtime_release_id())
        );
    }

    #[test]
    fn start_rebuilds_purged_docker_resources_from_the_verified_player_backup() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        let managed_game = runtime_root.join("game");
        let addon = temporary.path().join("addon");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::create_dir_all(&managed_game).expect("managed game");
        fs::create_dir_all(&addon).expect("addon");
        for filename in [
            "RealmBoxCompanions.lua",
            "RealmBoxCompanions.toc",
            "RealmBoxCompanions.xml",
        ] {
            fs::write(addon.join(filename), format!("fresh {filename}")).expect("addon fixture");
        }
        fs::write(
            &compose_file,
            "services:\n  server-data-init:\n    volumes:\n      - realmbox-server-data:/work\n    command:\n      - >-\n          /azerothcore/env/dist/bin/vmap4_extractor;\n          mkdir -p /work/vmaps;\n          /azerothcore/env/dist/bin/vmap4_assembler /work/Buildings /work/vmaps;\n          /azerothcore/env/dist/bin/mmaps_generator --config /azerothcore/env/dist/bin/mmaps-config.yaml --silent;\n  worldserver:\n    environment:\n      TEST: value\n",
        )
        .expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        let playerbots_config = runtime_root.join("server/env/dist/etc/modules/playerbots.conf");
        fs::create_dir_all(playerbots_config.parent().expect("module config"))
            .expect("module config");
        fs::write(playerbots_config, playerbots_fixture(false, 0, "")).expect("playerbots config");

        let backup_root = temporary.path().join(PLAYER_DATA_BACKUP_DIRECTORY);
        fs::create_dir_all(&backup_root).expect("backup root");
        let backup = backup_root.join("pre-migration-beforepurge.sql");
        fs::write(
            &backup,
            "-- Current Database: `acore_auth`\n-- Current Database: `acore_characters`\n-- Current Database: `acore_playerbots`\n-- Current Database: `acore_world`\n",
        )
        .expect("backup");
        fs::write(
            backup.with_extension("sha256"),
            format!("{}\n", sha256_file(&backup).expect("checksum")),
        )
        .expect("checksum");

        let runner = RecordingRunner {
            docker_volumes_present: Mutex::new(Some(false)),
            ..Default::default()
        };
        let mut service =
            LauncherService::new(temporary.path().to_path_buf(), addon, runner).expect("service");
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
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");

        let mut progress = Vec::new();
        let status = service
            .start(None, None, None, None, |event| progress.push(event))
            .expect("purged Docker recovery");

        assert_eq!(status.phase, LauncherPhase::Running);
        assert!(!temporary.path().join(DOCKER_RECOVERY_FILE).exists());
        assert!(progress.iter().any(|event| {
            event.phase == LauncherPhase::Recovering
                && event.message == "Restauration des personnages"
        }));
        let commands = service.runner.commands.lock().expect("commands");
        let database_start = commands
            .iter()
            .position(|command| command.contains("up -d --wait --wait-timeout 120 database"))
            .expect("database reconstruction");
        let restore = commands
            .iter()
            .position(|command| {
                command.contains("exec mysql") && command.contains("pre-migration-beforepurge.sql")
            })
            .expect("player backup restore");
        let migration = commands
            .iter()
            .position(|command| command.contains("run --rm db-import"))
            .expect("database migration");
        let client = commands
            .iter()
            .position(|command| command.starts_with(&client_executable.display().to_string()))
            .expect("client launch");
        assert!(database_start < restore);
        assert!(restore < migration);
        assert!(migration < client);
        assert!(
            commands
                .iter()
                .all(|command| !command.contains("--volumes"))
        );
    }

    #[test]
    fn running_playerbot_population_is_reloaded_after_launcher_restart() {
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
        fs::write(&module_config, playerbots_fixture(true, 5, "")).expect("playerbots config");
        let mut service = LauncherService::new(
            temporary.path().to_path_buf(),
            temporary.path().join("addon"),
            RecordingRunner {
                docker_memory: Mutex::new(Some("25769803776".into())),
                ..Default::default()
            },
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
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");
        let status = service
            .update_playerbot_population(true, 150, BotPresence::Natural)
            .expect("hot update");
        assert_eq!(status.phase, LauncherPhase::Running);
        assert_eq!(status.bot_count, 100);
        assert_eq!(status.requested_bot_count, 150);
        assert_eq!(status.applied_bot_count, 100);
        let config =
            fs::read_to_string(runtime_root.join("server/env/dist/etc/modules/playerbots.conf"))
                .expect("updated config");
        assert!(config.contains("AiPlayerbot.MaxRandomBots = 100"));
        let commands = service.runner.commands.lock().expect("commands");
        assert!(commands.iter().any(|command| {
            command.contains("exec -T worldserver sh -lc")
                && command.contains("reload config")
                && command.contains("playerbots rndbot reload")
                && command.contains("playerbots rndbot update")
        }));
    }

    #[test]
    fn playerbot_update_never_treats_docker_errors_or_a_missing_owned_world_as_stopped() {
        for scenario in ["ps-error", "owned-client-without-world"] {
            let temporary = tempfile::tempdir().expect("tempdir");
            let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
            let compose_file = runtime_root.join("server/compose.realmbox.yaml");
            let module_root = runtime_root.join("server/env/dist/etc/modules");
            fs::create_dir_all(&module_root).expect("module root");
            fs::write(&compose_file, "services: {}\n").expect("compose");
            fs::write(
                module_root.join("playerbots.conf.dist"),
                playerbots_fixture(true, 50, ""),
            )
            .expect("playerbots dist");
            let managed_config = module_root.join("playerbots.conf");
            fs::write(&managed_config, "original-config\n").expect("managed config");
            let runner = RecordingRunner::default();
            if scenario == "ps-error" {
                *runner.fail_next_ps.lock().expect("ps failure") = true;
            } else {
                *runner.empty_next_ps.lock().expect("empty ps") = true;
            }
            let mut service = LauncherService::new(
                temporary.path().to_path_buf(),
                temporary.path().join("addon"),
                runner,
            )
            .expect("service");
            service
                .save_record(&InstallationRecord {
                    schema_version: INSTALL_SCHEMA,
                    game_data_root: temporary.path().join("game-source"),
                    runtime_root: runtime_root.clone(),
                    client_executable: runtime_root.join("client/openwow-client"),
                    client_choice: ClientChoice::ManagedOpenWow,
                    compose_file,
                    bots_enabled: true,
                    bot_count: 50,
                    ai_enabled: false,
                    ai_model: None,
                    ollama_executable: None,
                    client_sha256: None,
                    ollama_sha256: None,
                    server_commit: SERVER_COMMIT.into(),
                    playerbots_commit: PLAYERBOTS_COMMIT.into(),
                    ollama_chat_commit: None,
                    runtime_release: Some(runtime_release_id()),
                })
                .expect("record");
            service.client_process_id = Some(42);
            let record_before =
                fs::read(temporary.path().join("installation.json")).expect("record before update");

            let error = service
                .update_playerbot_population(true, 100, BotPresence::Close)
                .expect_err("ambiguous Docker state must fail");
            if scenario == "ps-error" {
                assert!(error.contains("état du monde Docker indisponible"));
            } else {
                assert!(error.contains("client est ouvert"));
            }
            assert_eq!(
                fs::read_to_string(&managed_config).expect("managed config"),
                "original-config\n"
            );
            assert_eq!(
                fs::read(temporary.path().join("installation.json")).expect("record after update"),
                record_before
            );
            assert!(!temporary.path().join("world-preferences.json").exists());
            assert_eq!(service.client_process_id(), Some(42));
            assert!(
                service
                    .runner
                    .commands
                    .lock()
                    .expect("commands")
                    .iter()
                    .all(|command| !command.contains("exec -T worldserver"))
            );
        }
    }

    #[test]
    fn running_dialogue_chattiness_is_reloaded_without_stopping_the_world() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let runtime_root = temporary.path().join(RUNTIME_DIRECTORY);
        let compose_file = runtime_root.join("server/compose.realmbox.yaml");
        let client_executable = runtime_root.join("client/openwow-client");
        let dialogue_config =
            runtime_root.join("server/env/dist/etc/modules/mod_ollama_chat.conf.dist");
        fs::create_dir_all(compose_file.parent().expect("server")).expect("server");
        fs::create_dir_all(client_executable.parent().expect("client")).expect("client");
        fs::write(&compose_file, "services: {}").expect("compose");
        fs::write(&client_executable, "binary").expect("client");
        write_ollama_chat_fixture(&dialogue_config);
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
                bot_count: 50,
                ai_enabled: true,
                ai_model: Some("llama3.2:3b".into()),
                ollama_executable: Some(runtime_root.join("ai/ollama")),
                client_sha256: Some("test-openwow-sha256".into()),
                ollama_sha256: Some("test-ollama-sha256".into()),
                server_commit: SERVER_COMMIT.into(),
                playerbots_commit: PLAYERBOTS_COMMIT.into(),
                ollama_chat_commit: Some(OLLAMA_CHAT_COMMIT.into()),
                runtime_release: Some(runtime_release_id()),
            })
            .expect("record");
        service.client_process_id = Some(42);

        let status = service
            .configure_dialogue_chattiness(DialogueChattiness::Lively)
            .expect("live dialogue update");
        assert_eq!(status.phase, LauncherPhase::Running);
        assert_eq!(status.dialogue_chattiness, DialogueChattiness::Lively);
        let config = fs::read_to_string(
            runtime_root.join("server/env/dist/etc/modules/mod_ollama_chat.conf"),
        )
        .expect("updated config");
        assert!(config.contains("OllamaChat.EnableRandomChatter = 1"));
        assert!(config.contains("OllamaChat.BotReplyChance.Say = 35"));
        assert!(config.contains("OllamaChat.RateLimit.GlobalPerMinute = 6"));
        let commands = service.runner.commands.lock().expect("commands");
        assert!(commands.iter().any(|command| {
            command.contains("exec -T worldserver sh -lc")
                && command.contains("reload config")
                && command.contains("ollama reload")
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
            "INFO ready\nERROR playerbots failed in /Users/Benjamin/Games\nWARNING password=secret\n",
        )
        .expect("log");
        let entries = filtered_log_entries(&logs, 10).expect("entries");
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .any(|entry| entry.contains("playerbots failed"))
        );
        assert!(entries.iter().all(|entry| !entry.contains("Benjamin")));
        assert!(
            entries
                .iter()
                .any(|entry| entry.contains("ligne sensible masquée"))
        );
        assert_eq!(diagnose_entries(&entries, true).0, "bots");
        assert!(is_diagnostic_line("ERROR: failed to start"));
        assert!(!is_diagnostic_line("Converting Warningtree.m2"));
    }
}
