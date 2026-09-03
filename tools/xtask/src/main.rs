use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};

const MOD_OLLAMA_CHAT_PATCH: &str = "patches/mod-ollama-chat-realmbox.patch";

#[derive(Debug, PartialEq, Eq)]
struct PatchDeclaration {
    path: PathBuf,
    sha256: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    match command.as_str() {
        "doctor" => doctor(),
        "bootstrap" => run_command("pnpm", &["install"]),
        "dev" => run_command("pnpm", &["dev:preview"]),
        "build-launcher" => {
            run_command("pnpm", &["build"])?;
            run_command("cargo", &["build", "-p", "realmbox-desktop"])
        }
        "test" => {
            run_command("cargo", &["test", "--workspace"])?;
            run_command("pnpm", &["test"])
        }
        "verify" => run_command("pnpm", &["verify"]),
        "release" => match args.next().as_deref() {
            Some("check") => release_check(args.collect()),
            _ => bail!("usage: cargo xtask release check [--tag vX.Y.Z]"),
        },
        "build-openwow" | "build-server" | "build-runtimes" | "package" => {
            bail!(
                "{command} est sécurisé mais bloqué tant que les sources épinglées ou runtimes redistribuables ne sont pas disponibles; voir STATUS.md"
            )
        }
        _ => {
            println!(
                "RealmBox xtask\n\nCommandes: doctor, bootstrap, dev, build-launcher, test, verify, release check [--tag vX.Y.Z], build-openwow, build-server, build-runtimes, package"
            );
            Ok(())
        }
    }
}

fn release_check(args: Vec<String>) -> Result<()> {
    let root = workspace_root()?;
    let expected_tag = match args.as_slice() {
        [] => None,
        [flag, tag] if flag == "--tag" => Some(tag.as_str()),
        _ => bail!("usage: cargo xtask release check [--tag vX.Y.Z]"),
    };

    let third_party: toml::Value = read_toml(&root.join("third-party.lock.toml"))?;
    verify_mod_ollama_chat_patch(&root, &third_party)?;

    let cargo: toml::Value = read_toml(&root.join("Cargo.toml"))?;
    let version = cargo["workspace"]["package"]["version"]
        .as_str()
        .context("workspace.package.version absent de Cargo.toml")?;
    ensure_semver(version)?;

    for path in [
        "package.json",
        "apps/desktop/package.json",
        "site/package.json",
    ] {
        let document: serde_json::Value = read_json(&root.join(path))?;
        ensure!(
            document["version"].as_str() == Some(version),
            "{path}: version attendue {version}, trouvée {:?}",
            document["version"]
        );
    }

    let tauri: serde_json::Value = read_json(&root.join("apps/desktop/src-tauri/tauri.conf.json"))?;
    ensure!(
        tauri["version"].as_str() == Some(version),
        "apps/desktop/src-tauri/tauri.conf.json: version attendue {version}, trouvée {:?}",
        tauri["version"]
    );

    let manifest: serde_json::Value = read_json(&root.join("site/public/release-manifest.json"))?;
    ensure!(
        manifest["schemaVersion"].as_u64() == Some(1),
        "release-manifest: schemaVersion doit valoir 1"
    );
    ensure!(
        manifest["productVersion"].as_str() == Some(version),
        "release-manifest: productVersion doit valoir {version}"
    );
    ensure!(
        manifest["publicRelease"].is_null(),
        "release-manifest: publicRelease doit rester null tant qu'aucune release publique n'est prouvée"
    );
    ensure!(
        manifest["platforms"]["macosAppleSilicon"]["status"].as_str() == Some("qualified"),
        "release-manifest: statut macOS inattendu"
    );
    ensure!(
        manifest["platforms"]["windowsX64"]["status"].as_str() == Some("experimental"),
        "release-manifest: Windows ne peut pas être déclaré qualifié avant la fiche réelle"
    );

    let changelog =
        fs::read_to_string(root.join("CHANGELOG.md")).context("lecture de CHANGELOG.md")?;
    ensure!(
        changelog
            .lines()
            .any(|line| line.starts_with(&format!("## {version} "))),
        "CHANGELOG.md ne contient pas de section {version}"
    );
    let status = fs::read_to_string(root.join("STATUS.md")).context("lecture de STATUS.md")?;
    ensure!(
        status.contains(&format!("RealmBox {version}")),
        "STATUS.md ne mentionne pas RealmBox {version}"
    );
    let site = fs::read_to_string(root.join("site/src/components/Header.astro"))
        .context("lecture de site/src/components/Header.astro")?;
    ensure!(
        site.contains("manifest.productVersion"),
        "Header.astro n’affiche pas la version issue du manifeste"
    );

    if let Some(tag) = expected_tag {
        ensure!(
            tag == format!("v{version}"),
            "tag {tag} incohérent avec la version v{version}"
        );
    }

    println!("[ok] versions cohérentes: {version}");
    println!("[ok] patch mod-ollama-chat déclaré et vérifié");
    println!("[ok] aucune release publique revendiquée");
    println!("[ok] macOS Apple Silicon qualifié; Windows x64 expérimental");
    Ok(())
}

fn verify_mod_ollama_chat_patch(root: &Path, lock: &toml::Value) -> Result<()> {
    verify_mod_ollama_chat_patch_with(lock, |relative_path| {
        let path = root.join(relative_path);
        fs::read(&path).with_context(|| format!("lecture du patch déclaré {}", path.display()))
    })
}

fn verify_mod_ollama_chat_patch_with<F>(lock: &toml::Value, read_patch: F) -> Result<()>
where
    F: FnOnce(&Path) -> Result<Vec<u8>>,
{
    let declaration = mod_ollama_chat_patch_declaration(lock)?;
    let contents = read_patch(&declaration.path).with_context(|| {
        format!(
            "mod-ollama-chat: patch déclaré introuvable: {}",
            declaration.path.display()
        )
    })?;
    let actual_sha256 = sha256_hex(&contents);
    ensure!(
        actual_sha256 == declaration.sha256,
        "mod-ollama-chat: SHA-256 du patch incohérent (attendu {}, calculé {})",
        declaration.sha256,
        actual_sha256
    );
    Ok(())
}

fn mod_ollama_chat_patch_declaration(lock: &toml::Value) -> Result<PatchDeclaration> {
    let components = lock
        .get("component")
        .and_then(toml::Value::as_array)
        .context("third-party.lock.toml: tableau [[component]] absent")?;
    let matches = components
        .iter()
        .filter(|component| {
            component.get("name").and_then(toml::Value::as_str) == Some("mod-ollama-chat")
        })
        .collect::<Vec<_>>();
    ensure!(
        matches.len() == 1,
        "third-party.lock.toml: mod-ollama-chat doit être déclaré exactement une fois"
    );

    let component = matches[0];
    let patch_set = component
        .get("patch_set")
        .and_then(toml::Value::as_str)
        .context("third-party.lock.toml: mod-ollama-chat.patch_set absent")?;
    ensure!(
        patch_set != "none" && !patch_set.trim().is_empty(),
        "third-party.lock.toml: mod-ollama-chat doit déclarer son patch RealmBox"
    );
    ensure!(
        patch_set == MOD_OLLAMA_CHAT_PATCH,
        "third-party.lock.toml: chemin de patch mod-ollama-chat inattendu: {patch_set} (attendu {MOD_OLLAMA_CHAT_PATCH})"
    );

    let patch_sha256 = component
        .get("patch_sha256")
        .and_then(toml::Value::as_str)
        .context("third-party.lock.toml: mod-ollama-chat.patch_sha256 absent")?;
    ensure!(
        patch_sha256.len() == 64
            && patch_sha256
                .chars()
                .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character)),
        "third-party.lock.toml: mod-ollama-chat.patch_sha256 doit être un SHA-256 hexadécimal minuscule"
    );

    Ok(PatchDeclaration {
        path: PathBuf::from(patch_set),
        sha256: patch_sha256.to_owned(),
    })
}

fn sha256_hex(contents: &[u8]) -> String {
    format!("{:x}", Sha256::digest(contents))
}

fn workspace_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("racine du workspace introuvable")
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    let source =
        fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
    serde_json::from_str(&source).with_context(|| format!("JSON invalide dans {}", path.display()))
}

fn read_toml(path: &Path) -> Result<toml::Value> {
    let source =
        fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
    toml::from_str(&source).with_context(|| format!("TOML invalide dans {}", path.display()))
}

fn ensure_semver(version: &str) -> Result<()> {
    let parts = version.split('.').collect::<Vec<_>>();
    ensure!(
        parts.len() == 3
            && parts
                .iter()
                .all(|part| !part.is_empty()
                    && part.chars().all(|character| character.is_ascii_digit())),
        "version produit non SemVer simple: {version}"
    );
    Ok(())
}

fn doctor() -> Result<()> {
    println!("RealmBox development doctor");
    for (name, args, required) in [
        ("rustc", &["--version"][..], true),
        ("cargo", &["--version"][..], true),
        ("node", &["--version"][..], true),
        ("pnpm", &["--version"][..], true),
        ("cmake", &["--version"][..], false),
        ("pkg-config", &["--version"][..], false),
        ("ollama", &["--version"][..], false),
    ] {
        let status = Command::new(name).args(args).status();
        match status {
            Ok(exit) if exit.success() => println!("[ok] {name}"),
            _ if required => bail!("outil requis absent: {name}"),
            _ => println!("[optionnel absent] {name}"),
        }
    }
    println!(
        "[info] les spikes OpenWoW/AzerothCore exigent aussi CMake, pkg-config et leurs dépendances natives"
    );
    Ok(())
}

fn run_command(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("impossible de lancer {program}"))?;
    if !status.success() {
        bail!("{program} a échoué avec {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn lock_with_patch(patch_set: &str, patch_sha256: Option<&str>) -> toml::Value {
        let hash = patch_sha256
            .map(|value| format!("patch_sha256 = \"{value}\""))
            .unwrap_or_default();
        toml::from_str(&format!(
            r#"
                [[component]]
                name = "mod-ollama-chat"
                patch_set = "{patch_set}"
                {hash}
            "#
        ))
        .expect("fixture TOML valide")
    }

    #[test]
    fn accepts_declared_patch_with_matching_digest() {
        let lock = lock_with_patch(MOD_OLLAMA_CHAT_PATCH, Some(ABC_SHA256));

        verify_mod_ollama_chat_patch_with(&lock, |path| {
            assert_eq!(path, Path::new(MOD_OLLAMA_CHAT_PATCH));
            Ok(b"abc".to_vec())
        })
        .expect("le patch verrouillé doit être accepté");
    }

    #[test]
    fn rejects_mod_ollama_chat_without_patch() {
        let lock = lock_with_patch("none", None);

        let error = verify_mod_ollama_chat_patch_with(&lock, |_| Ok(Vec::new()))
            .expect_err("patch_set = none doit échouer");

        assert!(format!("{error:#}").contains("doit déclarer son patch RealmBox"));
    }

    #[test]
    fn rejects_missing_declared_patch() {
        let lock = lock_with_patch(MOD_OLLAMA_CHAT_PATCH, Some(ABC_SHA256));

        let error =
            verify_mod_ollama_chat_patch_with(&lock, |_| bail!("fichier volontairement absent"))
                .expect_err("un patch absent doit échouer");

        assert!(format!("{error:#}").contains("patch déclaré introuvable"));
    }

    #[test]
    fn rejects_drifted_declared_patch() {
        let lock = lock_with_patch(MOD_OLLAMA_CHAT_PATCH, Some(ABC_SHA256));

        let error = verify_mod_ollama_chat_patch_with(&lock, |_| Ok(b"abcd".to_vec()))
            .expect_err("un patch modifié doit échouer");

        assert!(format!("{error:#}").contains("SHA-256 du patch incohérent"));
    }

    #[test]
    fn rejects_unexpected_patch_path() {
        let lock = lock_with_patch("patches/other.patch", Some(ABC_SHA256));

        let error = verify_mod_ollama_chat_patch_with(&lock, |_| Ok(b"abc".to_vec()))
            .expect_err("un autre chemin de patch doit échouer");

        assert!(format!("{error:#}").contains("chemin de patch mod-ollama-chat inattendu"));
    }
}
