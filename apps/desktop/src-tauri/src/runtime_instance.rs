//! One native application instance per durable RealmBox data directory.
//!
//! The file stays in place permanently. Only its operating-system lock is
//! released on process exit, so a stale file after a crash is safe to reuse.
//! Acquire this guard before constructing the launcher and keep it for the
//! whole application lifetime, including while background commands are running.

use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
};

use fs2::FileExt;

const LOCK_FILE: &str = "runtime-instance.lock";
const ALREADY_RUNNING: &str = "RealmBox est déjà ouvert pour ce royaume. Fermez l’autre instance avant de continuer ; aucun nouveau royaume n’a été créé. RealmBox is already open for this realm; no new realm was created.";

/// Holding the file descriptor holds the lock; dropping it releases the lock.
pub struct RuntimeInstanceGuard {
    _lock: File,
}

impl RuntimeInstanceGuard {
    pub fn acquire(app_data: &Path) -> io::Result<Self> {
        if !app_data.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "le dossier persistant de RealmBox doit être un chemin absolu",
            ));
        }
        match fs::symlink_metadata(app_data) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "le dossier persistant de RealmBox n’est pas un dossier régulier",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir_all(app_data)?;
            }
            Err(error) => return Err(error),
        }
        let app_data_metadata = fs::symlink_metadata(app_data)?;
        if app_data_metadata.file_type().is_symlink() || !app_data_metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "le dossier persistant de RealmBox n’est pas un dossier régulier",
            ));
        }

        let path = app_data.join(LOCK_FILE);
        let existing = require_regular_lock_if_present(&path)?.is_some();
        let mut options = OpenOptions::new();
        // Never truncate, rename, remove, or replace an existing lock inode.
        options.read(true).write(true).truncate(false);
        if existing {
            options.create(false);
        } else {
            options.create_new(true);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = match options.open(&path) {
            Ok(lock) => lock,
            Err(error) if !existing && error.kind() == io::ErrorKind::AlreadyExists => {
                // Another instance may have created the permanent inode between
                // our inspection and create_new. Revalidate it before opening.
                require_regular_lock_if_present(&path)?.ok_or(error)?;
                OpenOptions::new().read(true).write(true).open(&path)?
            }
            Err(error) => return Err(error),
        };
        let path_metadata = require_regular_lock_if_present(&path)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "le verrou d’instance RealmBox a disparu pendant son ouverture",
            )
        })?;
        let opened_metadata = lock.metadata()?;
        if !opened_metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "le verrou d’instance RealmBox n’est pas un fichier régulier",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if path_metadata.dev() != opened_metadata.dev()
                || path_metadata.ino() != opened_metadata.ino()
            {
                return Err(io::Error::other(
                    "le fichier de verrou RealmBox a changé pendant son ouverture",
                ));
            }
        }
        #[cfg(not(unix))]
        let _ = path_metadata;

        FileExt::try_lock_exclusive(&lock).map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock
                || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
            {
                io::Error::new(io::ErrorKind::WouldBlock, ALREADY_RUNNING)
            } else {
                io::Error::new(
                    error.kind(),
                    format!("impossible de verrouiller l’instance RealmBox : {error}"),
                )
            }
        })?;
        Ok(Self { _lock: lock })
    }
}

fn require_regular_lock_if_present(path: &Path) -> io::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "le verrou d’instance RealmBox doit être un fichier régulier, sans lien symbolique",
            ))
        }
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_is_exclusive_and_released_without_removing_or_rewriting_its_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(LOCK_FILE);
        fs::write(&path, b"existing persistent lock file").unwrap();
        #[cfg(unix)]
        let original_inode = {
            use std::os::unix::fs::MetadataExt;
            fs::metadata(&path).unwrap().ino()
        };
        let first = RuntimeInstanceGuard::acquire(directory.path()).unwrap();
        let error = RuntimeInstanceGuard::acquire(directory.path())
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert!(error.to_string().contains("RealmBox est déjà ouvert"));
        assert!(error.to_string().contains("aucun nouveau royaume"));
        assert_eq!(fs::read(&path).unwrap(), b"existing persistent lock file");
        drop(first);
        assert!(path.is_file());
        let second = RuntimeInstanceGuard::acquire(directory.path()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"existing persistent lock file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(fs::metadata(&path).unwrap().ino(), original_inode);
        }
        drop(second);
        assert!(path.is_file());
    }

    #[test]
    fn missing_app_data_directory_is_created_without_any_realm_state() {
        let directory = tempfile::tempdir().unwrap();
        let app_data = directory.path().join("new-app-data");
        let guard = RuntimeInstanceGuard::acquire(&app_data).unwrap();
        assert!(app_data.join(LOCK_FILE).is_file());
        assert_eq!(fs::read_dir(&app_data).unwrap().count(), 1);
        drop(guard);
        assert!(app_data.join(LOCK_FILE).is_file());
    }

    #[test]
    fn invalid_lock_or_app_data_paths_are_refused() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join(LOCK_FILE)).unwrap();
        assert!(RuntimeInstanceGuard::acquire(directory.path()).is_err());
        assert!(RuntimeInstanceGuard::acquire(Path::new("relative-data")).is_err());
        let file = directory.path().join("not-a-directory");
        fs::write(&file, b"preserve").unwrap();
        assert!(RuntimeInstanceGuard::acquire(&file).is_err());
        assert_eq!(fs::read(file).unwrap(), b"preserve");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_lock_and_dangling_symlink_are_refused_without_touching_targets() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target");
        fs::write(&target, b"preserve").unwrap();
        let first_data = directory.path().join("first");
        fs::create_dir(&first_data).unwrap();
        symlink(&target, first_data.join(LOCK_FILE)).unwrap();
        assert!(RuntimeInstanceGuard::acquire(&first_data).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"preserve");

        let absent_target = directory.path().join("absent");
        let second_data = directory.path().join("second");
        fs::create_dir(&second_data).unwrap();
        symlink(&absent_target, second_data.join(LOCK_FILE)).unwrap();
        assert!(RuntimeInstanceGuard::acquire(&second_data).is_err());
        assert!(!absent_target.exists());

        let linked_data = directory.path().join("linked-data");
        symlink(&first_data, &linked_data).unwrap();
        assert!(RuntimeInstanceGuard::acquire(&linked_data).is_err());
    }

    #[test]
    fn lock_excludes_a_second_process_and_allows_it_after_release() {
        let directory = tempfile::tempdir().unwrap();
        let guard = RuntimeInstanceGuard::acquire(directory.path()).unwrap();
        let run_probe = |expected| {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "runtime_instance::tests::process_lock_probe"])
                .env("REALMBOX_INSTANCE_TEST_DIRECTORY", directory.path())
                .env("REALMBOX_INSTANCE_TEST_EXPECTED", expected)
                .output()
                .unwrap()
        };
        let blocked = run_probe("blocked");
        assert!(blocked.status.success(), "{:?}", blocked);
        drop(guard);
        let available = run_probe("available");
        assert!(available.status.success(), "{:?}", available);
        assert!(directory.path().join(LOCK_FILE).is_file());
    }

    #[test]
    fn process_lock_probe() {
        let Some(directory) = std::env::var_os("REALMBOX_INSTANCE_TEST_DIRECTORY") else {
            return;
        };
        let acquired = RuntimeInstanceGuard::acquire(Path::new(&directory));
        if std::env::var("REALMBOX_INSTANCE_TEST_EXPECTED").unwrap() == "blocked" {
            assert_eq!(acquired.err().unwrap().kind(), io::ErrorKind::WouldBlock);
        } else {
            assert!(acquired.is_ok());
        }
    }
}
