use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use anyhow::{Context, Result, bail, ensure};

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
    println!("[ok] aucune release publique revendiquée");
    println!("[ok] macOS Apple Silicon qualifié; Windows x64 expérimental");
    Ok(())
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
