//! Durable storage and crash recovery for solo-profile configuration changes.
//!
//! Snapshots and the transaction journal live under the durable application
//! data directory, never beside the replaceable worldserver configuration.
//! This module has no Docker or database effect. Its caller must stop the world
//! and hold the launcher's runtime-operation guard before mutating a profile.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::launcher::INSTALL_SCHEMA;
use crate::solo_profiles::{
    MutationOutcome, ProfileCatalog, ProfileSnapshot, ProfileState, ProfileSummary, SoloProfile,
    SoloProfileEngine,
};

const STORE_SCHEMA_VERSION: u32 = 1;
const STORE_DIRECTORY: &str = "solo-profiles-v1";
const MAX_CONFIG_BYTES: usize = 4 * 1024 * 1024;
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024;
const MAX_CONTROL_BYTES: usize = 16 * 1024;
const STAGING_PREFIX: &str = ".realmbox-solo-stage-";
static NEXT_STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoloProfileView {
    pub active_profile: Option<SoloProfile>,
    pub profiles: Vec<ProfileSummary>,
    pub rollback_available: bool,
    pub pending_change: bool,
}

#[derive(Debug, Clone)]
pub struct SoloProfileStore {
    app_data: PathBuf,
    config_path: PathBuf,
    installation_schema: u32,
    engine: SoloProfileEngine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct LatestRevision {
    schema_version: u32,
    revision: u64,
    snapshot_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ChangeOperation {
    Apply,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PendingChange {
    schema_version: u32,
    operation: ChangeOperation,
    revision: u64,
    snapshot_sha256: String,
    before_config_sha256: String,
    after_config_sha256: String,
    previous_latest: Option<LatestRevision>,
}

impl SoloProfileStore {
    pub fn new(
        app_data: &Path,
        config_path: &Path,
        installation_schema: u32,
    ) -> Result<Self, String> {
        if installation_schema != INSTALL_SCHEMA {
            return Err(format!(
                "schéma d’installation inconnu pour les profils solo : {installation_schema}"
            ));
        }
        if !app_data.is_absolute() || !config_path.is_absolute() {
            return Err("les chemins des profils solo doivent être absolus".to_owned());
        }
        let store_root = app_data.join(STORE_DIRECTORY);
        if config_path.starts_with(&store_root) {
            return Err(
                "la configuration ne peut pas se trouver dans les sauvegardes de profils"
                    .to_owned(),
            );
        }
        // The durable app-data root is not the replaceable runtime root. This
        // catches accidental calls with runtime-v3 itself as app_data.
        if app_data
            .components()
            .any(|component| component.as_os_str() == "runtime-v3")
        {
            return Err(
                "les snapshots de profils doivent rester hors du runtime remplaçable".to_owned(),
            );
        }
        let engine = SoloProfileEngine::new(
            INSTALL_SCHEMA,
            ProfileCatalog::realm_box_v1().map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        Ok(Self {
            app_data: app_data.to_owned(),
            config_path: config_path.to_owned(),
            installation_schema,
            engine,
        })
    }

    pub fn inspect(&self) -> Result<SoloProfileView, String> {
        self.validate_existing_directories()?;
        let source = self.read_config()?;
        let state = self.inspect_source(&source)?;
        let latest = self.load_latest()?;
        let pending = self.load_pending()?;
        if let Some(pending) = &pending {
            self.validate_pending_state(
                pending,
                &source,
                latest.as_ref().map(|(pointer, _)| pointer),
            )?;
        }
        self.view(
            &source,
            state,
            latest.as_ref().map(|(_, snapshot)| snapshot),
            pending.is_some(),
        )
    }

    pub fn apply(&self, profile: SoloProfile) -> Result<SoloProfileView, String> {
        let _guard = self.lock_store()?;
        self.require_no_pending()?;
        let source = self.read_config()?;
        let state = self.inspect_source(&source)?;
        let latest = self.load_latest()?;
        if state.active_profile == Some(profile) {
            return self.view(
                &source,
                state,
                latest.as_ref().map(|(_, snapshot)| snapshot),
                false,
            );
        }

        let revision = self.next_revision()?;
        let plan = self
            .engine
            .plan_transition(&state, profile, revision)
            .map_err(|error| error.to_string())?;
        let update = self
            .engine
            .apply_plan_to_config(&source, &plan)
            .map_err(|error| error.to_string())?;
        if update.outcome != MutationOutcome::Applied {
            return self.inspect();
        }

        let pointer = self.persist_snapshot(&plan.snapshot)?;
        let pending = PendingChange {
            schema_version: STORE_SCHEMA_VERSION,
            operation: ChangeOperation::Apply,
            revision,
            snapshot_sha256: pointer.snapshot_sha256,
            before_config_sha256: sha256(source.as_bytes()),
            after_config_sha256: sha256(update.contents.as_bytes()),
            previous_latest: latest.map(|(pointer, _)| pointer),
        };
        self.persist_pending(&pending)?;
        self.finish_pending(&pending)?;
        self.inspect()
    }

    pub fn rollback(&self) -> Result<SoloProfileView, String> {
        let _guard = self.lock_store()?;
        self.require_no_pending()?;
        let source = self.read_config()?;
        let state = self.inspect_source(&source)?;
        let Some((pointer, snapshot)) = self.load_latest()? else {
            return self.view(&source, state, None, false);
        };
        let update = self
            .engine
            .rollback_config(&source, &snapshot)
            .map_err(|error| error.to_string())?;
        if update.outcome == MutationOutcome::AlreadyInRequestedState {
            return self.view(&source, state, Some(&snapshot), false);
        }

        let pending = PendingChange {
            schema_version: STORE_SCHEMA_VERSION,
            operation: ChangeOperation::Rollback,
            revision: pointer.revision,
            snapshot_sha256: pointer.snapshot_sha256.clone(),
            before_config_sha256: sha256(source.as_bytes()),
            after_config_sha256: sha256(update.contents.as_bytes()),
            previous_latest: Some(pointer),
        };
        self.persist_pending(&pending)?;
        self.finish_pending(&pending)?;
        self.inspect()
    }

    /// Completes only an already journaled transaction. The current file must
    /// match the complete before or after hash, so unrelated edits made during
    /// an interruption are never overwritten. Repeated resumes are no-ops.
    pub fn resume_pending(&self) -> Result<(), String> {
        self.validate_existing_directories()?;
        if self.load_pending()?.is_none() {
            return Ok(());
        }
        let _guard = self.lock_store()?;
        if let Some(pending) = self.load_pending()? {
            self.finish_pending(&pending)?;
        }
        Ok(())
    }

    fn root(&self) -> PathBuf {
        self.app_data.join(STORE_DIRECTORY)
    }

    fn snapshots_dir(&self) -> PathBuf {
        self.root().join("snapshots")
    }

    fn snapshot_path(&self, revision: u64) -> PathBuf {
        self.snapshots_dir()
            .join(format!("revision-{revision:020}.json"))
    }

    fn latest_path(&self) -> PathBuf {
        self.root().join("latest.json")
    }

    fn pending_path(&self) -> PathBuf {
        self.root().join("pending.json")
    }

    fn read_config(&self) -> Result<String, String> {
        let bytes = read_regular_file(&self.config_path, MAX_CONFIG_BYTES)?;
        String::from_utf8(bytes)
            .map_err(|_| "worldserver.conf n’est pas un texte UTF-8 valide".to_owned())
    }

    fn inspect_source(&self, source: &str) -> Result<ProfileState, String> {
        self.engine
            .inspect_config(self.installation_schema, source)
            .map_err(|error| error.to_string())
    }

    fn view(
        &self,
        source: &str,
        state: ProfileState,
        latest: Option<&ProfileSnapshot>,
        pending_change: bool,
    ) -> Result<SoloProfileView, String> {
        let rollback_available = !pending_change
            && latest.is_some_and(|snapshot| {
                self.engine
                    .rollback_config(source, snapshot)
                    .is_ok_and(|update| update.outcome == MutationOutcome::Applied)
            });
        Ok(SoloProfileView {
            active_profile: state.active_profile,
            profiles: self
                .engine
                .profile_summaries()
                .map_err(|error| error.to_string())?,
            rollback_available,
            pending_change,
        })
    }

    fn validate_existing_directories(&self) -> Result<(), String> {
        require_directory_if_present(&self.app_data)?;
        require_directory_if_present(&self.root())?;
        require_directory_if_present(&self.snapshots_dir())
    }

    fn lock_store(&self) -> Result<File, String> {
        self.validate_existing_directories()?;
        require_directory(&self.app_data)?;
        create_private_directory(&self.root())?;
        create_private_directory(&self.snapshots_dir())?;
        let lock_path = self.root().join("operation.lock");
        if fs::symlink_metadata(&lock_path).is_ok() {
            require_regular_file(&lock_path)?;
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options
            .open(&lock_path)
            .map_err(|error| error.to_string())?;
        require_regular_file(&lock_path)?;
        FileExt::try_lock_exclusive(&lock)
            .map_err(|_| "une autre modification de profil solo est en cours".to_owned())?;
        Ok(lock)
    }

    fn load_latest(&self) -> Result<Option<(LatestRevision, ProfileSnapshot)>, String> {
        let Some(bytes) = read_optional_regular_file(&self.latest_path(), MAX_CONTROL_BYTES)?
        else {
            return Ok(None);
        };
        let pointer: LatestRevision = decode_json(&bytes, "pointeur de profil")?;
        validate_pointer(&pointer)?;
        let snapshot = self.load_snapshot(pointer.revision, &pointer.snapshot_sha256)?;
        Ok(Some((pointer, snapshot)))
    }

    fn load_snapshot(
        &self,
        revision: u64,
        expected_sha256: &str,
    ) -> Result<ProfileSnapshot, String> {
        if revision == 0 || !valid_sha256(expected_sha256) {
            return Err("référence de snapshot de profil invalide".to_owned());
        }
        let bytes = read_regular_file(&self.snapshot_path(revision), MAX_SNAPSHOT_BYTES)?;
        if sha256(&bytes) != expected_sha256 {
            return Err("le SHA-256 du snapshot de profil ne correspond pas".to_owned());
        }
        let text =
            std::str::from_utf8(&bytes).map_err(|_| "snapshot de profil non UTF-8".to_owned())?;
        let snapshot = ProfileSnapshot::decode(text, self.installation_schema)
            .map_err(|error| error.to_string())?;
        if snapshot.revision != revision {
            return Err("la révision du snapshot ne correspond pas à son nom".to_owned());
        }
        Ok(snapshot)
    }

    fn next_revision(&self) -> Result<u64, String> {
        let mut maximum = 0_u64;
        for entry in fs::read_dir(self.snapshots_dir()).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.starts_with("revision-") {
                continue;
            }
            let revision = parse_snapshot_name(name)?;
            let bytes = read_regular_file(&entry.path(), MAX_SNAPSHOT_BYTES)?;
            self.load_snapshot(revision, &sha256(&bytes))?;
            maximum = maximum.max(revision);
        }
        maximum
            .checked_add(1)
            .ok_or_else(|| "nombre maximal de révisions de profils atteint".to_owned())
    }

    fn persist_snapshot(&self, snapshot: &ProfileSnapshot) -> Result<LatestRevision, String> {
        let encoded = snapshot.encode().map_err(|error| error.to_string())?;
        let validated = ProfileSnapshot::decode(&encoded, self.installation_schema)
            .map_err(|error| error.to_string())?;
        if validated != *snapshot {
            return Err("le snapshot de profil préparé diffère".to_owned());
        }
        let bytes = encoded.as_bytes();
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err("snapshot de profil trop volumineux".to_owned());
        }
        let pointer = LatestRevision {
            schema_version: STORE_SCHEMA_VERSION,
            revision: snapshot.revision,
            snapshot_sha256: sha256(bytes),
        };
        publish_new_private(&self.snapshot_path(snapshot.revision), bytes)?;
        let read_back = self.load_snapshot(pointer.revision, &pointer.snapshot_sha256)?;
        if read_back != *snapshot {
            return Err("la relecture du snapshot de profil diffère".to_owned());
        }
        Ok(pointer)
    }

    fn load_pending(&self) -> Result<Option<PendingChange>, String> {
        let Some(bytes) = read_optional_regular_file(&self.pending_path(), MAX_CONTROL_BYTES)?
        else {
            return Ok(None);
        };
        let pending: PendingChange = decode_json(&bytes, "journal de profil")?;
        validate_pending(&pending)?;
        self.load_snapshot(pending.revision, &pending.snapshot_sha256)?;
        Ok(Some(pending))
    }

    fn require_no_pending(&self) -> Result<(), String> {
        if self.load_pending()?.is_some() {
            return Err("une modification de profil interrompue doit être reprise avant une nouvelle action".to_owned());
        }
        Ok(())
    }

    fn persist_pending(&self, pending: &PendingChange) -> Result<(), String> {
        validate_pending(pending)?;
        self.load_snapshot(pending.revision, &pending.snapshot_sha256)?;
        let encoded = serde_json::to_vec_pretty(pending).map_err(|error| error.to_string())?;
        let validated: PendingChange = decode_json(&encoded, "journal de profil préparé")?;
        if validated != *pending {
            return Err("le journal de profil préparé diffère".to_owned());
        }
        publish_new_private(&self.pending_path(), &encoded)?;
        let read_back = self
            .load_pending()?
            .ok_or_else(|| "journal de profil absent après écriture".to_owned())?;
        if read_back != *pending {
            return Err("la relecture du journal de profil diffère".to_owned());
        }
        Ok(())
    }

    fn validate_pending_state(
        &self,
        pending: &PendingChange,
        source: &str,
        latest: Option<&LatestRevision>,
    ) -> Result<(), String> {
        validate_pending(pending)?;
        self.load_snapshot(pending.revision, &pending.snapshot_sha256)?;
        let target = pending.target_pointer();
        if latest != pending.previous_latest.as_ref() && latest != Some(&target) {
            return Err(
                "le pointeur de profil a changé pendant l’opération interrompue".to_owned(),
            );
        }
        let current_hash = sha256(source.as_bytes());
        if current_hash != pending.before_config_sha256
            && current_hash != pending.after_config_sha256
        {
            return Err(
                "worldserver.conf a changé pendant l’opération interrompue ; reprise refusée"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn finish_pending(&self, pending: &PendingChange) -> Result<(), String> {
        let snapshot = self.load_snapshot(pending.revision, &pending.snapshot_sha256)?;
        let source = self.read_config()?;
        let latest = self.load_latest()?;
        self.validate_pending_state(
            pending,
            &source,
            latest.as_ref().map(|(pointer, _)| pointer),
        )?;

        if sha256(source.as_bytes()) == pending.before_config_sha256 {
            let update = match pending.operation {
                ChangeOperation::Apply => self.engine.apply_snapshot_config(&source, &snapshot),
                ChangeOperation::Rollback => self.engine.rollback_config(&source, &snapshot),
            }
            .map_err(|error| error.to_string())?;
            if sha256(update.contents.as_bytes()) != pending.after_config_sha256 {
                return Err("le résultat prévu ne correspond pas au journal de profil".to_owned());
            }
            atomic_replace(
                &self.config_path,
                update.contents.as_bytes(),
                Some(source.as_bytes()),
                true,
            )?;
        }

        let after = self.read_config()?;
        if sha256(after.as_bytes()) != pending.after_config_sha256 {
            return Err(
                "la configuration de profil appliquée n’a pas été relue à l’identique".to_owned(),
            );
        }
        let proof = match pending.operation {
            ChangeOperation::Apply => self.engine.apply_snapshot_config(&after, &snapshot),
            ChangeOperation::Rollback => self.engine.rollback_config(&after, &snapshot),
        }
        .map_err(|error| error.to_string())?;
        if proof.outcome != MutationOutcome::AlreadyInRequestedState {
            return Err("le profil relu ne correspond pas à l’état terminal attendu".to_owned());
        }

        let target = pending.target_pointer();
        let current_pointer_bytes =
            read_optional_regular_file(&self.latest_path(), MAX_CONTROL_BYTES)?;
        let current_pointer = current_pointer_bytes
            .as_deref()
            .map(|bytes| decode_json::<LatestRevision>(bytes, "pointeur de profil"))
            .transpose()?;
        if current_pointer.as_ref() != pending.previous_latest.as_ref()
            && current_pointer.as_ref() != Some(&target)
        {
            return Err("le pointeur de profil a changé avant la finalisation".to_owned());
        }
        if current_pointer.as_ref() != Some(&target) {
            let encoded = serde_json::to_vec_pretty(&target).map_err(|error| error.to_string())?;
            atomic_replace(
                &self.latest_path(),
                &encoded,
                current_pointer_bytes.as_deref(),
                false,
            )?;
        }
        let confirmed = self
            .load_latest()?
            .ok_or_else(|| "pointeur de profil absent après finalisation".to_owned())?;
        if confirmed.0 != target {
            return Err("le pointeur final du profil ne correspond pas".to_owned());
        }

        let current_pending = self
            .load_pending()?
            .ok_or_else(|| "journal de profil disparu avant finalisation".to_owned())?;
        if current_pending != *pending {
            return Err("le journal de profil a changé avant finalisation".to_owned());
        }
        fs::remove_file(self.pending_path()).map_err(|error| error.to_string())?;
        sync_directory(&self.root())
    }
}

impl PendingChange {
    fn target_pointer(&self) -> LatestRevision {
        LatestRevision {
            schema_version: STORE_SCHEMA_VERSION,
            revision: self.revision,
            snapshot_sha256: self.snapshot_sha256.clone(),
        }
    }
}

fn validate_pointer(pointer: &LatestRevision) -> Result<(), String> {
    if pointer.schema_version != STORE_SCHEMA_VERSION {
        return Err(format!(
            "schéma de pointeur de profil inconnu : {}",
            pointer.schema_version
        ));
    }
    if pointer.revision == 0 || !valid_sha256(&pointer.snapshot_sha256) {
        return Err("pointeur de profil invalide".to_owned());
    }
    Ok(())
}

fn validate_pending(pending: &PendingChange) -> Result<(), String> {
    if pending.schema_version != STORE_SCHEMA_VERSION {
        return Err(format!(
            "schéma de journal de profil inconnu : {}",
            pending.schema_version
        ));
    }
    if pending.revision == 0
        || !valid_sha256(&pending.snapshot_sha256)
        || !valid_sha256(&pending.before_config_sha256)
        || !valid_sha256(&pending.after_config_sha256)
    {
        return Err("journal de profil invalide".to_owned());
    }
    if let Some(previous) = &pending.previous_latest {
        validate_pointer(previous)?;
    }
    if pending.operation == ChangeOperation::Rollback
        && pending.previous_latest.as_ref() != Some(&pending.target_pointer())
    {
        return Err("le journal de retour arrière ne référence pas le dernier snapshot".to_owned());
    }
    Ok(())
}

fn parse_snapshot_name(name: &str) -> Result<u64, String> {
    let digits = name
        .strip_prefix("revision-")
        .and_then(|name| name.strip_suffix(".json"))
        .ok_or_else(|| "nom de snapshot de profil non reconnu".to_owned())?;
    if digits.len() != 20 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("nom de snapshot de profil non reconnu".to_owned());
    }
    let revision = digits.parse::<u64>().map_err(|error| error.to_string())?;
    if revision == 0 {
        return Err("révision de snapshot nulle".to_owned());
    }
    Ok(revision)
}

fn decode_json<T: for<'de> Deserialize<'de>>(bytes: &[u8], label: &str) -> Result<T, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("{label} illisible : {error}"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "le dossier de profils n’est pas un dossier régulier : {}",
            path.display()
        ));
    }
    Ok(())
}

fn require_directory_if_present(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) => require_directory(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn create_private_directory(path: &Path) -> Result<(), String> {
    match fs::create_dir(path) {
        Ok(()) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                    .map_err(|error| error.to_string())?;
            }
            sync_directory(
                path.parent()
                    .ok_or_else(|| "dossier parent absent".to_owned())?,
            )?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => require_directory(path),
        Err(error) => Err(error.to_string()),
    }
}

fn require_regular_file(path: &Path) -> Result<fs::Metadata, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "le fichier de profils n’est pas un fichier régulier : {}",
            path.display()
        ));
    }
    Ok(metadata)
}

fn read_regular_file(path: &Path, maximum: usize) -> Result<Vec<u8>, String> {
    let metadata = require_regular_file(path)?;
    if metadata.len() > maximum as u64 {
        return Err(format!(
            "fichier de profils trop volumineux : {}",
            path.display()
        ));
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() > maximum {
        return Err(format!(
            "fichier de profils trop volumineux : {}",
            path.display()
        ));
    }
    Ok(bytes)
}

fn read_optional_regular_file(path: &Path, maximum: usize) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => read_regular_file(path, maximum).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// A preparation name is never authoritative. A process crash may leave this
/// file behind, but neither snapshot discovery nor journal recovery reads it.
struct PrivateStagingFile {
    path: PathBuf,
}

impl Drop for PrivateStagingFile {
    fn drop(&mut self) {
        // This object is constructed only after create_new succeeds, so it
        // never removes a colliding file that belongs to another operation.
        let _ = fs::remove_file(&self.path);
    }
}

fn stage_private_file(parent: &Path, bytes: &[u8]) -> Result<PrivateStagingFile, String> {
    require_directory(parent)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    for _ in 0..16 {
        let sequence = NEXT_STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            "{STAGING_PREFIX}{}-{nonce}-{sequence}.tmp",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = match options.open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        };
        let staged = PrivateStagingFile { path };
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        if read_regular_file(&staged.path, bytes.len())? != bytes {
            return Err("la relecture du fichier privé préparé diffère".to_owned());
        }
        sync_directory(parent)?;
        return Ok(staged);
    }
    Err("impossible de réserver un fichier de préparation privé".to_owned())
}

/// Publishes a fully written and verified inode without replacing any existing
/// final name. Rust's hard_link uses link(2) on Unix and CreateHardLink on
/// Windows. Preparation and destination share a parent/filesystem; a filesystem
/// without hard-link support fails closed instead of falling back to a copy.
fn publish_new_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "dossier parent absent".to_owned())?;
    let staged = stage_private_file(parent, bytes)?;
    fs::hard_link(&staged.path, path).map_err(|error| error.to_string())?;
    sync_directory(parent)?;
    if read_regular_file(path, bytes.len())? != bytes {
        return Err("la relecture du fichier privé publié diffère".to_owned());
    }
    Ok(())
}

fn atomic_replace(
    path: &Path,
    bytes: &[u8],
    expected: Option<&[u8]>,
    preserve_permissions: bool,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "dossier parent absent".to_owned())?;
    require_directory(parent)?;
    let current = read_optional_regular_file(path, MAX_CONFIG_BYTES)?;
    if current.as_deref() != expected {
        return Err("le fichier a changé avant son remplacement atomique".to_owned());
    }
    let permissions = if preserve_permissions {
        Some(require_regular_file(path)?.permissions())
    } else {
        None
    };
    let staged = stage_private_file(parent, bytes)?;
    if let Some(permissions) = permissions {
        fs::set_permissions(&staged.path, permissions).map_err(|error| error.to_string())?;
    }
    if read_optional_regular_file(path, MAX_CONFIG_BYTES)?.as_deref() != expected {
        return Err("le fichier a changé pendant la préparation atomique".to_owned());
    }
    fs::rename(&staged.path, path).map_err(|error| error.to_string())?;
    sync_directory(parent)?;
    if read_regular_file(path, bytes.len())? != bytes {
        return Err("le fichier atomique n’a pas été relu à l’identique".to_owned());
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| error.to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solo_profiles::{ManagedSetting, SettingValue};

    fn fixture() -> (tempfile::TempDir, SoloProfileStore) {
        let directory = tempfile::tempdir().unwrap();
        let app_data = directory.path().join("app-data");
        let runtime = directory.path().join("runtime");
        fs::create_dir(&app_data).unwrap();
        fs::create_dir(&runtime).unwrap();
        let config = runtime.join("worldserver.conf");
        fs::write(&config, normal_config()).unwrap();
        let store = SoloProfileStore::new(&app_data, &config, 3).unwrap();
        (directory, store)
    }

    fn normal_config() -> String {
        "# custom header\n\
Rate.XP.Kill = 1\n\
Rate.XP.Quest = 1\n\
Rate.XP.Quest.DF = 1\n\
Rate.XP.Explore = 1\n\
Rate.XP.Pet = 1\n\
Rate.Reputation.Gain = 1\n\
Rate.Drop.Money = 1\n\
MaxPrimaryTradeSkill = 2\n\
Quests.IgnoreRaid = 0\n\
Instance.IgnoreLevel = 0\n\
Instance.IgnoreRaid = 0\n\
Unmanaged.Value = keep\n"
            .to_owned()
    }

    fn prepare_interrupted_apply(store: &SoloProfileStore) -> (PendingChange, String) {
        prepare_interrupted_apply_with_engine(store, &store.engine)
    }

    fn prepare_interrupted_apply_with_engine(
        store: &SoloProfileStore,
        engine: &SoloProfileEngine,
    ) -> (PendingChange, String) {
        let _guard = store.lock_store().unwrap();
        let source = store.read_config().unwrap();
        let state = engine
            .inspect_config(store.installation_schema, &source)
            .unwrap();
        let plan = engine
            .plan_transition(
                &state,
                SoloProfile::Comfortable,
                store.next_revision().unwrap(),
            )
            .unwrap();
        let update = engine.apply_plan_to_config(&source, &plan).unwrap();
        let pointer = store.persist_snapshot(&plan.snapshot).unwrap();
        let pending = PendingChange {
            schema_version: STORE_SCHEMA_VERSION,
            operation: ChangeOperation::Apply,
            revision: pointer.revision,
            snapshot_sha256: pointer.snapshot_sha256,
            before_config_sha256: sha256(source.as_bytes()),
            after_config_sha256: sha256(update.contents.as_bytes()),
            previous_latest: None,
        };
        store.persist_pending(&pending).unwrap();
        (pending, update.contents)
    }

    #[test]
    fn inspect_is_read_only_and_apply_rollback_preserve_unmanaged_content() {
        let (_directory, store) = fixture();
        let view = store.inspect().unwrap();
        assert_eq!(view.active_profile, Some(SoloProfile::Normal));
        assert!(!view.rollback_available);
        assert!(!store.root().exists());

        let view = store.apply(SoloProfile::Comfortable).unwrap();
        assert_eq!(view.active_profile, Some(SoloProfile::Comfortable));
        assert!(view.rollback_available);
        assert!(!view.pending_change);
        assert!(
            store
                .read_config()
                .unwrap()
                .contains("Unmanaged.Value = keep")
        );
        assert!(store.read_config().unwrap().contains("# custom header"));

        let view = store.rollback().unwrap();
        assert_eq!(view.active_profile, Some(SoloProfile::Normal));
        assert!(!view.rollback_available);
        assert_eq!(store.read_config().unwrap(), normal_config());
        assert_eq!(store.rollback().unwrap(), view);
    }

    #[test]
    fn snapshots_are_complete_private_and_never_overwritten() {
        let (_directory, store) = fixture();
        store.apply(SoloProfile::Comfortable).unwrap();
        let first_path = store.snapshot_path(1);
        let first = fs::read(&first_path).unwrap();
        let first_snapshot =
            ProfileSnapshot::decode(std::str::from_utf8(&first).unwrap(), 3).unwrap();
        assert_eq!(first_snapshot.previous_values.len(), 11);
        assert_eq!(first_snapshot.applied_values.len(), 11);
        assert!(store.persist_snapshot(&first_snapshot).is_err());

        store.apply(SoloProfile::Accelerated).unwrap();
        assert_eq!(fs::read(&first_path).unwrap(), first);
        assert!(store.snapshot_path(2).is_file());
        assert!(
            !store
                .snapshot_path(1)
                .starts_with(store.config_path.parent().unwrap())
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(first_path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn resumes_interruption_before_config_write_and_is_idempotent() {
        let (_directory, store) = fixture();
        prepare_interrupted_apply(&store);
        assert!(store.inspect().unwrap().pending_change);
        assert!(store.apply(SoloProfile::Accelerated).is_err());
        store.resume_pending().unwrap();
        assert_eq!(
            store.inspect().unwrap().active_profile,
            Some(SoloProfile::Comfortable)
        );
        assert!(!store.pending_path().exists());
        store.resume_pending().unwrap();
        assert_eq!(store.load_latest().unwrap().unwrap().0.revision, 1);
    }

    #[test]
    fn resumes_apply_and_rollback_from_an_older_catalog_snapshot() {
        let (_directory, store) = fixture();
        let mut profiles = ProfileCatalog::realm_box_v1().unwrap().profiles;
        profiles
            .iter_mut()
            .find(|definition| definition.profile == SoloProfile::Comfortable)
            .unwrap()
            .values
            .insert(ManagedSetting::XpKill, SettingValue::RateMilli(1_500));
        let legacy_engine =
            SoloProfileEngine::new(INSTALL_SCHEMA, ProfileCatalog::new(7, profiles).unwrap())
                .unwrap();
        let (pending, expected) = prepare_interrupted_apply_with_engine(&store, &legacy_engine);
        let snapshot = store
            .load_snapshot(pending.revision, &pending.snapshot_sha256)
            .unwrap();
        assert_eq!(snapshot.catalog_version, 7);
        assert_ne!(
            snapshot.applied_values,
            ProfileCatalog::realm_box_v1()
                .unwrap()
                .definition(SoloProfile::Comfortable)
                .unwrap()
                .values
        );

        store.resume_pending().unwrap();

        assert_eq!(store.read_config().unwrap(), expected);
        assert!(!store.pending_path().exists());
        assert_eq!(store.load_latest().unwrap().unwrap().0.revision, 1);
        let view = store.inspect().unwrap();
        assert_eq!(view.active_profile, None);
        assert!(view.rollback_available);

        let (pointer, snapshot) = store.load_latest().unwrap().unwrap();
        let source = store.read_config().unwrap();
        let rollback = store.engine.rollback_config(&source, &snapshot).unwrap();
        let rollback_pending = PendingChange {
            schema_version: STORE_SCHEMA_VERSION,
            operation: ChangeOperation::Rollback,
            revision: pointer.revision,
            snapshot_sha256: pointer.snapshot_sha256.clone(),
            before_config_sha256: sha256(source.as_bytes()),
            after_config_sha256: sha256(rollback.contents.as_bytes()),
            previous_latest: Some(pointer),
        };
        store.persist_pending(&rollback_pending).unwrap();
        store.resume_pending().unwrap();
        assert_eq!(store.read_config().unwrap(), normal_config());
        assert!(!store.pending_path().exists());
        let view = store.inspect().unwrap();
        assert_eq!(view.active_profile, Some(SoloProfile::Normal));
        assert!(!view.rollback_available);
    }

    #[test]
    fn resume_rejects_a_tampered_snapshot_before_mutating_config() {
        let (_directory, store) = fixture();
        let (pending, _) = prepare_interrupted_apply(&store);
        let original = store.read_config().unwrap();
        let mut snapshot = store
            .load_snapshot(pending.revision, &pending.snapshot_sha256)
            .unwrap();
        snapshot
            .applied_values
            .insert(ManagedSetting::XpKill, SettingValue::RateMilli(1_500));
        fs::write(
            store.snapshot_path(pending.revision),
            snapshot.encode().unwrap(),
        )
        .unwrap();

        assert!(store.resume_pending().unwrap_err().contains("SHA-256"));
        assert_eq!(store.read_config().unwrap(), original);
        assert!(store.pending_path().exists());
        assert!(!store.latest_path().exists());
    }

    #[test]
    fn resumes_after_config_write_before_pointer_publication() {
        let (_directory, store) = fixture();
        let (_pending, after) = prepare_interrupted_apply(&store);
        fs::write(&store.config_path, &after).unwrap();
        assert!(!store.latest_path().exists());
        store.resume_pending().unwrap();
        assert_eq!(store.read_config().unwrap(), after);
        assert!(store.inspect().unwrap().rollback_available);
        assert_eq!(store.load_latest().unwrap().unwrap().0.revision, 1);
    }

    #[test]
    fn resume_without_journal_does_not_read_config_or_create_store() {
        let directory = tempfile::tempdir().unwrap();
        let app_data = directory.path().join("not-created");
        let config = directory.path().join("missing-worldserver.conf");
        let store = SoloProfileStore::new(&app_data, &config, INSTALL_SCHEMA).unwrap();
        store.resume_pending().unwrap();
        assert!(!app_data.exists());
        assert!(!config.exists());
        assert!(
            SoloProfileStore::new(&app_data, &config, 999)
                .unwrap_err()
                .contains("inconnu")
        );
    }

    #[test]
    fn resumes_after_pointer_publication_without_rewriting_config() {
        let (_directory, store) = fixture();
        let (pending, after) = prepare_interrupted_apply(&store);
        fs::write(&store.config_path, &after).unwrap();
        fs::write(
            store.latest_path(),
            serde_json::to_vec_pretty(&pending.target_pointer()).unwrap(),
        )
        .unwrap();
        store.resume_pending().unwrap();
        assert_eq!(store.read_config().unwrap(), after);
        assert!(!store.pending_path().exists());
        assert!(store.inspect().unwrap().rollback_available);
    }

    #[test]
    fn interrupted_rollback_resumes_after_config_write() {
        let (_directory, store) = fixture();
        store.apply(SoloProfile::Comfortable).unwrap();
        let (pointer, snapshot) = store.load_latest().unwrap().unwrap();
        let source = store.read_config().unwrap();
        let after = store
            .engine
            .rollback_config(&source, &snapshot)
            .unwrap()
            .contents;
        let pending = PendingChange {
            schema_version: STORE_SCHEMA_VERSION,
            operation: ChangeOperation::Rollback,
            revision: pointer.revision,
            snapshot_sha256: pointer.snapshot_sha256.clone(),
            before_config_sha256: sha256(source.as_bytes()),
            after_config_sha256: sha256(after.as_bytes()),
            previous_latest: Some(pointer),
        };
        store.persist_pending(&pending).unwrap();
        fs::write(&store.config_path, after).unwrap();
        store.resume_pending().unwrap();
        let view = store.inspect().unwrap();
        assert_eq!(view.active_profile, Some(SoloProfile::Normal));
        assert!(!view.rollback_available);
        assert!(!view.pending_change);
    }

    #[test]
    fn unknown_journal_and_external_config_drift_fail_closed() {
        let (_directory, store) = fixture();
        let (pending, _) = prepare_interrupted_apply(&store);
        let original = store.read_config().unwrap();
        let mut unknown = pending.clone();
        unknown.schema_version = 99;
        fs::write(store.pending_path(), serde_json::to_vec(&unknown).unwrap()).unwrap();
        assert!(store.resume_pending().unwrap_err().contains("inconnu"));
        assert_eq!(store.read_config().unwrap(), original);

        fs::write(store.pending_path(), serde_json::to_vec(&pending).unwrap()).unwrap();
        let external = format!("{original}External.Edit = preserve\n");
        fs::write(&store.config_path, &external).unwrap();
        assert!(store.resume_pending().unwrap_err().contains("changé"));
        assert_eq!(store.read_config().unwrap(), external);
        assert!(store.pending_path().exists());
    }

    #[test]
    fn rollback_availability_requires_exact_managed_state_but_preserves_other_files() {
        let (_directory, store) = fixture();
        store.apply(SoloProfile::Comfortable).unwrap();
        let unrelated = store.root().join("notes.txt");
        fs::write(&unrelated, "user-owned notes").unwrap();
        let unrelated_snapshot = store.snapshots_dir().join("readme.txt");
        fs::write(&unrelated_snapshot, "not managed").unwrap();
        let drifted = store
            .read_config()
            .unwrap()
            .replace("Rate.XP.Kill = 2", "Rate.XP.Kill = 1.5");
        fs::write(&store.config_path, drifted).unwrap();
        assert!(!store.inspect().unwrap().rollback_available);
        assert!(store.rollback().is_err());
        store.apply(SoloProfile::Accelerated).unwrap();
        assert_eq!(fs::read_to_string(unrelated).unwrap(), "user-owned notes");
        assert_eq!(
            fs::read_to_string(unrelated_snapshot).unwrap(),
            "not managed"
        );
    }

    #[test]
    fn completed_transaction_allows_unmanaged_edits_to_survive_rollback() {
        let (_directory, store) = fixture();
        store.apply(SoloProfile::Comfortable).unwrap();
        let source = format!(
            "{}External.Option = preserve\n",
            store.read_config().unwrap()
        );
        fs::write(&store.config_path, source).unwrap();
        assert!(store.inspect().unwrap().rollback_available);
        store.rollback().unwrap();
        assert!(
            store
                .read_config()
                .unwrap()
                .contains("External.Option = preserve")
        );
    }

    #[test]
    fn orphan_snapshot_is_not_overwritten_and_store_lock_blocks_concurrent_changes() {
        let (_directory, store) = fixture();
        {
            let _guard = store.lock_store().unwrap();
            let source = store.read_config().unwrap();
            let state = store.inspect_source(&source).unwrap();
            let plan = store
                .engine
                .plan_transition(&state, SoloProfile::Comfortable, 1)
                .unwrap();
            store.persist_snapshot(&plan.snapshot).unwrap();
            assert!(store.apply(SoloProfile::Comfortable).is_err());
        }
        let orphan = fs::read(store.snapshot_path(1)).unwrap();
        store.apply(SoloProfile::Accelerated).unwrap();
        assert_eq!(fs::read(store.snapshot_path(1)).unwrap(), orphan);
        assert_eq!(store.load_latest().unwrap().unwrap().0.revision, 2);
    }

    #[test]
    fn corrupt_snapshot_and_unknown_pointer_are_not_treated_as_empty_state() {
        let (_directory, store) = fixture();
        store.apply(SoloProfile::Comfortable).unwrap();
        let original = store.read_config().unwrap();
        fs::write(store.snapshot_path(1), b"{}").unwrap();
        assert!(store.inspect().unwrap_err().contains("SHA-256"));
        assert!(store.apply(SoloProfile::Accelerated).is_err());
        assert_eq!(store.read_config().unwrap(), original);

        let unknown = serde_json::json!({
            "schemaVersion": 99,
            "revision": 1,
            "snapshotSha256": "0".repeat(64)
        });
        fs::write(store.latest_path(), serde_json::to_vec(&unknown).unwrap()).unwrap();
        assert!(store.inspect().unwrap_err().contains("inconnu"));
    }

    #[test]
    fn interrupted_preparations_are_ignored_without_exposing_partial_final_files() {
        let (_directory, store) = fixture();
        let original = store.read_config().unwrap();
        let (partial_journal, partial_snapshot, complete_snapshot_stage) = {
            let _guard = store.lock_store().unwrap();
            let partial_journal = store
                .root()
                .join(format!("{STAGING_PREFIX}journal-cut.tmp"));
            let partial_snapshot = store
                .snapshots_dir()
                .join(format!("{STAGING_PREFIX}snapshot-cut.tmp"));
            fs::write(&partial_journal, b"{\"schemaVersion\":").unwrap();
            fs::write(&partial_snapshot, b"{\"revision\":").unwrap();
            let state = store.inspect_source(&original).unwrap();
            let plan = store
                .engine
                .plan_transition(&state, SoloProfile::Comfortable, 1)
                .unwrap();
            let staged = stage_private_file(
                &store.snapshots_dir(),
                plan.snapshot.encode().unwrap().as_bytes(),
            )
            .unwrap();
            let complete_snapshot_stage = staged.path.clone();
            std::mem::forget(staged); // Simulate process death before publication.
            (partial_journal, partial_snapshot, complete_snapshot_stage)
        };

        assert!(!store.pending_path().exists());
        assert!(!store.snapshot_path(1).exists());
        store.resume_pending().unwrap();
        assert_eq!(store.read_config().unwrap(), original);
        assert!(!store.inspect().unwrap().pending_change);
        store.apply(SoloProfile::Comfortable).unwrap();
        assert_eq!(store.load_latest().unwrap().unwrap().0.revision, 1);
        assert_eq!(fs::read(partial_journal).unwrap(), b"{\"schemaVersion\":");
        assert_eq!(fs::read(partial_snapshot).unwrap(), b"{\"revision\":");
        assert!(complete_snapshot_stage.is_file());
    }

    #[test]
    fn published_complete_journal_recovers_when_staging_cleanup_was_interrupted() {
        let (_directory, store) = fixture();
        let (_pending, after) = prepare_interrupted_apply(&store);
        let encoded = fs::read(store.pending_path()).unwrap();
        fs::remove_file(store.pending_path()).unwrap();
        let staged = stage_private_file(&store.root(), &encoded).unwrap();
        assert!(!store.pending_path().exists());

        // This is the publisher's single final-name operation. A crash on
        // either side sees no journal or the already complete journal.
        fs::hard_link(&staged.path, store.pending_path()).unwrap();
        sync_directory(&store.root()).unwrap();
        let leftover = staged.path.clone();
        std::mem::forget(staged); // Simulate process death before stage removal.
        assert_eq!(fs::read(store.pending_path()).unwrap(), encoded);
        assert!(store.inspect().unwrap().pending_change);

        store.resume_pending().unwrap();
        assert_eq!(store.read_config().unwrap(), after);
        assert!(!store.pending_path().exists());
        assert_eq!(fs::read(leftover).unwrap(), encoded);
        assert!(store.inspect().unwrap().rollback_available);
    }

    #[test]
    fn atomic_new_publication_never_overwrites_and_cleans_only_its_own_stage() {
        let directory = tempfile::tempdir().unwrap();
        let final_path = directory.path().join("final.json");
        let unrelated_stage = directory
            .path()
            .join(format!("{STAGING_PREFIX}unrelated.tmp"));
        fs::write(&final_path, b"original").unwrap();
        fs::write(&unrelated_stage, b"keep").unwrap();

        assert!(publish_new_private(&final_path, b"replacement").is_err());
        assert_eq!(fs::read(&final_path).unwrap(), b"original");
        assert_eq!(fs::read(&unrelated_stage).unwrap(), b"keep");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);

        let new_path = directory.path().join("new.json");
        publish_new_private(&new_path, b"complete payload").unwrap();
        assert_eq!(fs::read(new_path).unwrap(), b"complete payload");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 3);
    }

    #[test]
    fn corruption_under_a_final_name_still_fails_closed() {
        let (_directory, store) = fixture();
        {
            let _guard = store.lock_store().unwrap();
        }
        let original = store.read_config().unwrap();
        fs::write(store.pending_path(), b"{\"schemaVersion\":").unwrap();
        assert!(store.resume_pending().is_err());
        assert_eq!(store.read_config().unwrap(), original);

        fs::remove_file(store.pending_path()).unwrap();
        fs::write(store.snapshot_path(1), b"{\"revision\":").unwrap();
        assert!(store.apply(SoloProfile::Comfortable).is_err());
        assert_eq!(store.read_config().unwrap(), original);
        assert!(store.snapshot_path(1).is_file());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlinked_configuration_and_snapshot_directory() {
        use std::os::unix::fs::symlink;

        let (directory, store) = fixture();
        let alternate = directory.path().join("alternate.conf");
        fs::write(&alternate, normal_config()).unwrap();
        fs::remove_file(&store.config_path).unwrap();
        symlink(&alternate, &store.config_path).unwrap();
        assert!(store.apply(SoloProfile::Comfortable).is_err());
        assert_eq!(fs::read_to_string(&alternate).unwrap(), normal_config());

        let (_directory, store) = fixture();
        fs::create_dir(store.root()).unwrap();
        symlink(store.config_path.parent().unwrap(), store.snapshots_dir()).unwrap();
        assert!(store.apply(SoloProfile::Comfortable).is_err());
    }
}
