use std::{
    env,
    process::{Command, ExitCode},
};

use anyhow::{Context, Result, bail};

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
    let command = env::args().nth(1).unwrap_or_else(|| "help".into());
    match command.as_str() {
        "doctor" => doctor(),
        "bootstrap" => run_command("pnpm", &["install"]),
        "dev" => run_command("pnpm", &["dev:fake"]),
        "build-launcher" => {
            run_command("pnpm", &["build"])?;
            run_command("cargo", &["build", "-p", "realmbox-desktop"])
        }
        "test" => {
            run_command("cargo", &["test", "--workspace"])?;
            run_command("pnpm", &["test"])
        }
        "verify" => run_command("pnpm", &["verify"]),
        "build-openwow" | "build-server" | "build-runtimes" | "package" => {
            bail!(
                "{command} est sécurisé mais bloqué tant que les sources épinglées ou runtimes redistribuables ne sont pas disponibles; voir STATUS.md"
            )
        }
        _ => {
            println!(
                "RealmBox xtask\n\nCommandes: doctor, bootstrap, dev, build-launcher, test, verify, build-openwow, build-server, build-runtimes, package"
            );
            Ok(())
        }
    }
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
