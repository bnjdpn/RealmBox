# RealmBox

[Français](README.md) · [English](README.en.md)

[![Common validation](https://github.com/bnjdpn/RealmBox/actions/workflows/validation.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/validation.yml)
[![macOS arm64](https://github.com/bnjdpn/RealmBox/actions/workflows/macos-arm64.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/macos-arm64.yml)
[![Windows x64](https://github.com/bnjdpn/RealmBox/actions/workflows/windows-x64.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/windows-x64.yml)
[![RealmBox website](https://github.com/bnjdpn/RealmBox/actions/workflows/pages.yml/badge.svg)](https://bnjdpn.github.io/RealmBox/?lang=en)
[![AGPL-3.0 license](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

RealmBox is a Windows x64 and Apple Silicon macOS launcher for a fully local 3.3.5a world. On first launch, it asks for a compatible game-data folder owned by the player, then prepares the client, local database, AzerothCore server, and Playerbots. On capable machines it can also run Ollama and `mod-ollama-chat` for locally generated companion dialogue.

The 0.2.4 interface is available in French and English. It separates **My world**, **Companions**, **Dialogue**, and **Diagnostics**, shows a short cause with a useful recovery action, and keeps server details out of the player flow.

The launcher and website share an original northern-fantasy portal identity: two distinct raster panoramas created for RealmBox with ImageGen and an R/B medallion. The launcher is a fixed 1024 × 640 scene centered on the current status and one primary action; language, companions, dialogue, and diagnostics stay in settings. No game asset or remote font is used. The repository documents the [design system](docs/DESIGN_SYSTEM.md), [historical reinterpretation audit](docs/WOTLK_ERA_VISUAL_AUDIT.md), and [provenance of every visual asset](docs/ASSET_PROVENANCE.md).

## Download and tutorial

- [RealmBox website and English tutorial](https://bnjdpn.github.io/RealmBox/?lang=en)
- [macOS arm64 and Windows x64 previews](https://github.com/bnjdpn/RealmBox/releases)
- [Factual status and evidence limits](STATUS.md)

Current artifacts are unsigned previews. Always verify the `SHA256SUMS.txt` file attached to the release. The Apple Silicon macOS path has been qualified locally; the Windows x64 installer is built in CI, but the complete Windows 11 path still needs testing.

## First launch

1. Start Docker Desktop.
2. Choose **OpenWoW managed by RealmBox** (recommended), or on Windows x64 choose a player-provided original client.
3. Select a legally obtained 3.3.5a build 12340 folder containing `Data`.
4. Enable or disable Playerbots and choose a population of 5, 25, 50, 100, or 150. RealmBox caps it against the memory actually assigned to Docker.
5. Optionally enable local dialogue after reviewing the model, size, and speed RealmBox decided from CanIRun. The model is not selected manually.
6. Select **Install**.

RealmBox validates real MPQ signatures and the required WotLK and locale archives before installation. With the recommended option, it downloads the official OpenWoW 0.1.2 release for the platform and verifies its SHA-256 digest. It never downloads proprietary game data.

After installation, **Settings → Game client** shows the folder in use and lets the player choose another one while the world is stopped. RealmBox validates the 3.3.5a archives again, atomically rebuilds OpenWoW’s data links or updates the `Wow.exe` path, without reinstalling the server or touching the character database.

RealmBox submits only the processor name, core count, and memory size to CanIRun. It evaluates a closed allowlist, keeps comfortable candidates inside a 25% RAM budget capped at 8 GB, then automatically selects the best estimated speed per official download GB. A 1B model is only a fallback when no 3B+ candidate is comfortable. Before downloading, RealmBox shows the selected model and exact Ollama size; after pulling it, RealmBox verifies the manifest against its pinned digest.

After initial setup, the **Dialogue** view can enable the feature on demand without reinstalling the realm. Download starts only after confirmation. Disabling keeps the local model for a fast later re-enable, and the world must be closed while this configuration changes.

Player releases pull four multi-architecture, digest-pinned server images instead of compiling AzerothCore on the player's machine. Their linux-amd64 and linux-arm64 manifests were built from immutable commits and successfully pulled without registry authentication in GitHub Actions. Binary publication remains a preview until the complete transitive notice audit and real Windows path are finished.

## Updates without character loss

RealmBox refuses to reinstall over an existing realm and its Docker commands are forbidden from deleting persistent volumes. Before the first migration run by each RealmBox version, it automatically exports all four local databases, verifies that the dump includes accounts and characters, and writes a SHA-256 checksum. The backup stays outside the runtime in `player-data-backups` and is never overwritten. If that proof fails, migration and startup stop without advancing the version marker. See the [update contract](docs/UPDATES.md).

## Runtime

When the player chooses **Play**, RealmBox starts:

```text
local MySQL → backup for a new version → server-data check → migrations → optional local Ollama → authserver/worldserver → selected client
```

Game ports bind only to `127.0.0.1`; MySQL is not published to the host. RealmBox supervises the client process and shuts down the world, database, and Ollama when the owned client exits. Player data remains on the player's machine.

The in-game RealmBox addon can create a balanced party of four level-matched bots next to the player, then issue bounded follow, attack, stay, regroup, and leave commands. Its FR/EN panel collapses into a draggable minimap icon; panel visibility, icon position, and panel position are remembered. `/realmbox` or `/rb` also toggles it. The panel reports the party composition visible to the client, disables impossible commands, and explicitly controls the Playerbots strong-ability strategy. The remaining bots continue roaming the world autonomously.

While playing, the **Companions** view can change the requested population without closing the client. RealmBox first recalculates the memory cap, reloads `playerbots.conf` through the official Playerbots command, then triggers a full update. Actual bot connections and disconnections may take a moment.

## Client by platform

- **Windows x64**: a compatible copy containing `Wow.exe` can run directly; OpenWoW remains an experimental alternative.
- **Apple Silicon macOS**: the Windows package supplies the data but not a native executable. RealmBox uses OpenWoW arm64 instead of requiring a Windows VM.
- **Linux**: planned but not implemented. Native OpenWoW and the Windows client through Wine both need real-path qualification.

See [ROADMAP.md](ROADMAP.md) and [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) for the tracked product work and client matrix.

## Development

The currently observed development host is Apple Silicon macOS. Windows x64 has a dedicated CI build and still requires a full manual smoke test on Windows.

```sh
pnpm install
pnpm dev          # Tauri application with real commands
pnpm dev:preview  # browser-only UI preview
pnpm verify
python3 -m http.server 1421 --directory site  # Pages website preview
```

See [STATUS.md](STATUS.md) for the evidence split between automated tests, builds, manual checks, and the full real path. See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## Legal and security

RealmBox is licensed under AGPL-3.0-only. No proprietary game data, extracted server data, user database, credential, or model may be committed, attached to an issue, or uploaded to a release. Binary redistribution remains gated by the transitive license and notice audit described in [docs/LEGAL_AND_DISTRIBUTION.md](docs/LEGAL_AND_DISTRIBUTION.md).

Security reports should follow [SECURITY.md](SECURITY.md).
