# RealmBox

[Français](README.md) · [English](README.en.md)

[![Common validation](https://github.com/bnjdpn/RealmBox/actions/workflows/validation.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/validation.yml)
[![macOS arm64](https://github.com/bnjdpn/RealmBox/actions/workflows/macos-arm64.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/macos-arm64.yml)
[![Windows x64](https://github.com/bnjdpn/RealmBox/actions/workflows/windows-x64.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/windows-x64.yml)
[![AGPL-3.0 license](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

RealmBox is a Windows x64 and Apple Silicon macOS launcher for a fully local 3.3.5a world. On first launch, it asks for a compatible game-data folder owned by the player, then prepares the client, local database, AzerothCore server, and Playerbots. On capable machines it can also run Ollama and `mod-ollama-chat` for locally generated companion dialogue.

The interface is an original Wrath-era MMO launcher composition. RealmBox contains no Blizzard logo, artwork, copy, game binary, or game data.

## First launch

1. Start Docker Desktop.
2. Choose **OpenWoW managed by RealmBox** (recommended), or on Windows x64 choose a player-provided original client.
3. Select a legally obtained 3.3.5a build 12340 folder containing `Data`.
4. Enable or disable Playerbots and choose a population of 5, 25, 50, 100, or 150. RealmBox caps it against the memory actually assigned to Docker.
5. Optionally enable local AI dialogue when CanIRun reports a comfortable model.
6. Select **Install**.

RealmBox validates real MPQ signatures and the required WotLK and locale archives before installation. With the recommended option, it downloads the official OpenWoW 0.1.2 release for the platform and verifies its SHA-256 digest. It never downloads proprietary game data.

Player releases are designed to pull four multi-architecture, digest-pinned server images instead of compiling AzerothCore on the player's machine. Until those images and their complete notices pass the release gate, local development bundles deliberately retain the source-build fallback and are not presented as production releases.

## Runtime

On later launches RealmBox starts:

```text
local MySQL → server-data check → migrations → optional local Ollama → authserver/worldserver → selected client
```

Game ports bind only to `127.0.0.1`; MySQL is not published to the host. RealmBox supervises the client process and shuts down the world, database, and Ollama when the owned client exits. Player data remains on the player's machine.

The in-game RealmBox addon can create a balanced party of four level-matched bots next to the player, then issue bounded follow, attack, stay, regroup, and leave commands. The remaining bots continue roaming the world autonomously.

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
```

See [STATUS.md](STATUS.md) for the evidence split between automated tests, builds, manual checks, and the full real path. See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## Legal and security

RealmBox is licensed under AGPL-3.0-only. No proprietary game data, extracted server data, user database, credential, or model may be committed, attached to an issue, or uploaded to a release. Binary redistribution remains gated by the transitive license and notice audit described in [docs/LEGAL_AND_DISTRIBUTION.md](docs/LEGAL_AND_DISTRIBUTION.md).

Security reports should follow [SECURITY.md](SECURITY.md).
