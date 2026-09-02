use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("lecture ou écriture de configuration impossible: {0}")]
    Io(#[from] std::io::Error),
    #[error("la configuration n'a pas de dossier parent")]
    MissingParent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditResult {
    pub contents: String,
    pub changed_keys: Vec<String>,
}

pub fn edit_line_aware(input: &str, updates: &BTreeMap<String, String>) -> EditResult {
    let newline = if input.contains("\r\n") { "\r\n" } else { "\n" };
    let ended_with_newline = input.ends_with('\n');
    let mut remaining = updates.clone();
    let mut changed_keys = Vec::new();
    let mut output = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.is_empty() {
            output.push(line.to_owned());
            continue;
        }
        let Some((left, old_value)) = line.split_once('=') else {
            output.push(line.to_owned());
            continue;
        };
        let key = left.trim();
        if let Some(new_value) = remaining.remove(key) {
            let leading = &left[..left.len() - left.trim_start().len()];
            let spacing_before_equals = &left[key.len() + leading.len()..];
            let value_prefix_len = old_value.len() - old_value.trim_start().len();
            let value_prefix = &old_value[..value_prefix_len];
            output.push(format!(
                "{leading}{key}{spacing_before_equals}={value_prefix}{new_value}"
            ));
            changed_keys.push(key.to_owned());
        } else {
            output.push(line.to_owned());
        }
    }
    for (key, value) in remaining {
        output.push(format!("{key} = {value}"));
        changed_keys.push(key);
    }
    let mut contents = output.join(newline);
    if ended_with_newline || !contents.is_empty() {
        contents.push_str(newline);
    }
    EditResult {
        contents,
        changed_keys,
    }
}

pub fn write_atomically_with_backup(path: &Path, contents: &str) -> Result<PathBuf, ConfigError> {
    let parent = path.parent().ok_or(ConfigError::MissingParent)?;
    fs::create_dir_all(parent)?;
    let backup = path.with_extension("realmbox.bak");
    if path.exists() {
        fs::copy(path, &backup)?;
    }
    let temporary = path.with_extension("realmbox.tmp");
    {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&temporary, path)?;
    Ok(backup)
}

pub fn redact_diff_value(key: &str, value: &str) -> String {
    let normalized = key.to_ascii_lowercase();
    if ["password", "secret", "token", "connectionstring"]
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        "<masqué>".into()
    } else {
        value.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_comments_unknown_keys_order_and_crlf() {
        let input = "# joueur\r\nRealm = old\r\nUnknown = keep me\r\n";
        let updates = BTreeMap::from([
            ("Realm".into(), "127.0.0.1".into()),
            ("Locale".into(), "frFR".into()),
        ]);
        let edited = edit_line_aware(input, &updates);
        assert_eq!(
            edited.contents,
            "# joueur\r\nRealm = 127.0.0.1\r\nUnknown = keep me\r\nLocale = frFR\r\n"
        );
    }

    #[test]
    fn writes_atomically_and_keeps_backup() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("monde.conf");
        fs::write(&path, "before").expect("fixture");
        let backup = write_atomically_with_backup(&path, "after").expect("write");
        assert_eq!(fs::read_to_string(path).expect("new"), "after");
        assert_eq!(fs::read_to_string(backup).expect("backup"), "before");
    }

    #[test]
    fn redacts_sensitive_values() {
        assert_eq!(
            redact_diff_value("Database.Password", "hunter2"),
            "<masqué>"
        );
        assert_eq!(redact_diff_value("Realm", "localhost"), "localhost");
    }
}
