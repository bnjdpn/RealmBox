# RealmBox

**World of Warcraft. Azeroth on your computer. Bots included.**

[Website](https://bnjdpn.github.io/RealmBox/en/) · [Releases](https://github.com/bnjdpn/RealmBox/releases) · [Installation](docs/INSTALLATION.md) · [Français](README.fr.md)

RealmBox is an open-source desktop launcher for playing World of Warcraft locally. Give it the `Data` folder from a compatible WoW client and it prepares a private AzerothCore realm, populates Azeroth with Playerbots, launches the game, and supervises the complete local runtime.

![RealmBox ready to launch World of Warcraft locally](site/public/assets/launcher-ready-fr.webp)

## What the project provides

- one player-oriented application for setup, start, stop, configuration, and diagnostics;
- a local AzerothCore authentication and world server with MySQL;
- autonomous Playerbots that quickly gather a same-faction majority within sight of active players, plus a controllable companion party in game;
- managed OpenWoW on Apple Silicon macOS, plus `Wow.exe` or OpenWoW on Windows x64;
- optional, rate-limited local player-to-bot and bot-to-bot dialogue powered by a RealmBox-managed Ollama runtime;
- atomic installation, immutable server images, persistent character data, and verified pre-migration backups.

RealmBox contains no World of Warcraft client, MPQ archive, extracted map, credential, character database, or other proprietary game data. Those files are read from the player's own compatible copy and remain local.

## Architecture

```text
React 19 + TypeScript
        │ narrow Tauri commands
        ▼
Rust LauncherService ── typed platform/runtime interfaces
        ├── OpenWoW or player-owned Wow.exe
        ├── local extraction from the player's read-only Data folder
        ├── Docker Compose project: realmbox-v3
        │     ├── MySQL
        │     ├── AzerothCore authserver
        │     ├── AzerothCore worldserver + mod-playerbots
        │     └── database import and extraction tools
        ├── RealmBox Companions addon
        └── optional local Ollama process
```

The React interface never controls processes, Docker, or secrets directly. Tauri exposes a small command surface backed by `LauncherService`; platform and runtime effects go through typed interfaces that can be replaced with fakes in macOS tests.

The normal lifecycle is:

```text
choose Data → validate MPQs → stage runtime → extract locally → import databases
→ publish installation atomically → Play → start services → launch WoW
→ owned client exits → stop services without deleting volumes
```

Server images and third-party source revisions are pinned in [`third-party.lock.toml`](third-party.lock.toml). The installation manifest is published only after every required component has been verified.

## Game client

RealmBox accepts the client root or its `Data` folder. The technical compatibility target is WoW 3.3.5a build 12340; the launcher validates the expected MPQ files, detects the locale, and lets the AzerothCore extraction tools confirm the exact build. This identifier is a client compatibility constraint, not the product description.

ChromieCraft offers download pages in both languages:

- [French client and downloads](https://chromiecraft.com/fr/telechargements/)
- [English client and downloads](https://chromiecraft.com/en/downloads/)

Choose the client or language pack offered on the page you want. RealmBox ultimately uses the locale actually present in `Data`. On Apple Silicon Macs, the downloaded Windows package supplies game data while RealmBox launches managed native OpenWoW. On Windows x64, the player's `Wow.exe` is the preferred path when present; managed OpenWoW remains available as an option.

## Requirements

- Apple Silicon Mac or Windows x64 PC;
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) installed and running;
- at least 24 GiB of free disk space, plus space for the optional local dialogue model;
- a complete compatible WoW `Data` folder;
- internet access for the first installation.

The bot ceiling follows the memory assigned to Docker, not total system memory:

| Docker memory | Maximum autonomous bots |
| --- | ---: |
| Under 12 GiB | 5 |
| 12–19 GiB | 50 |
| 20–27 GiB | 100 |
| 28 GiB or more | 150 |

## Installation

1. Install and start [Docker Desktop](https://www.docker.com/products/docker-desktop/).
2. Download the compatible WoW data from the [French](https://chromiecraft.com/fr/telechargements/) or [English](https://chromiecraft.com/en/downloads/) ChromieCraft page.
3. Download RealmBox from [GitHub Releases](https://github.com/bnjdpn/RealmBox/releases) and compare the artifact with `SHA256SUMS.txt`.
4. Open RealmBox and select the folder containing `Data`.
5. Choose the bot population and optional local dialogue, then select **Install**.
6. Select **Play** when RealmBox reports that Azeroth is ready.

Current distributed binaries are not signed or notarized. Do not bypass an operating-system warning unless the downloaded artifact's SHA-256 matches the published checksum. See the [complete installation guide](docs/INSTALLATION.md) for platform details.

## Persistence and update safety

The Docker Compose project name is permanently `realmbox-v3`; it is not an application version. Keeping it stable preserves the player database volume across application releases.

Before the first migration performed by each desktop version, RealmBox:

1. exports all expected MySQL databases with a consistent dump;
2. verifies the dump contents and SHA-256;
3. stores the backup outside the replaceable runtime without overwriting an existing backup;
4. applies the migration;
5. advances the migrated-version marker only after success.

An unknown installation schema, an incomplete backup, or a failed migration stops the update. The launcher never turns an existing realm into a fresh installation and never invokes `docker compose down --volumes` or `-v`.

## Repository layout

| Path | Purpose |
| --- | --- |
| `apps/desktop/` | React interface and Tauri desktop application |
| `apps/desktop/src-tauri/` | Rust launcher state machine and platform/runtime integrations |
| `addons/RealmBoxCompanions/` | In-game companion controls |
| `runtime/` | Compose templates, manifests, and platform helpers |
| `patches/` | Reviewed patches applied to pinned upstream components |
| `site/` | Astro source for the bilingual GitHub Pages website |
| `scripts/` | Release, screenshot, manifest, and validation tooling |
| `tools/xtask/` | Repository and release invariant checks |
| `docs/` | Architecture, installation, compatibility, security, and operations |

## Development

The workspace uses pnpm, Node.js, Rust, and Tauri. Docker Desktop is required for the real local runtime path.

```sh
pnpm install
pnpm dev:preview   # React interface with the fake runtime
pnpm dev           # Tauri desktop application
pnpm site:dev      # GitHub Pages site
pnpm verify        # UI, scripts, build, site, Rust, and release invariants
```

Useful focused commands:

```sh
pnpm typecheck
pnpm test
pnpm site:build
cargo test --workspace
cargo xtask release check
```

## Technical documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Installation](docs/INSTALLATION.md)
- [Compatibility](docs/COMPATIBILITY.md)
- [Updates and backups](docs/UPDATES.md)
- [Security](docs/SECURITY.md)
- [Building](docs/BUILDING.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Distribution and licenses](docs/LEGAL_AND_DISTRIBUTION.md)

## License and independence

RealmBox is distributed under [AGPL-3.0-only](LICENSE). It is an independent project and is not affiliated with or endorsed by Blizzard Entertainment or ChromieCraft. World of Warcraft, Azeroth, and related names belong to their respective owners.
