# RealmBox

**World of Warcraft. Azeroth on your computer. Bots included.**

[Website](https://bnjdpn.github.io/RealmBox/en/) · [Releases](https://github.com/bnjdpn/RealmBox/releases) · [Installation](docs/INSTALLATION.md) · [Français](README.fr.md)

RealmBox is an open-source desktop launcher for playing World of Warcraft locally. Give it the `Data` folder from a compatible WoW client and it prepares a private AzerothCore realm, populates Azeroth with Playerbots, launches the game, and supervises the complete local runtime.

![RealmBox ready to launch World of Warcraft locally](site/public/assets/launcher-ready-en.webp)

*RealmBox 0.5.0 interface, captured with demonstration data.*

## What’s new in 0.5.0

- **Guided setup:** choose your WoW copy, configure companions, then review the installation. Docker and disk checks must pass before downloading.
- **Realm shortcuts:** open companions, dialogue, solo profiles, Protection, and the Local guide from the home screen. The population shown is configured, not a live online count.
- **Your adventure pace:** choose Normal, Comfortable, or Accelerated with an exact preview and a way to restore the previous rules.
- **Local quest and item lookup:** search the existing world catalogue by name, with up to eight results and visible sources, without AI or access to character data.
- **Companion squad presets:** three five-player presets, an observed primary companion, explicit party/target scope, and a command preview.
- **Runtime recovery:** a single-instance guard and a bounded local-dialogue failure recovery patch; the latter requires rebuilt server images.

**Availability, checked 4 September 2026:** [0.5.0 is a public preview](https://github.com/bnjdpn/RealmBox/releases/tag/v0.5.0), with no installers attached at the time of this update. Check that page for current assets and `SHA256SUMS.txt` before installing. Full 0.5.0 gameplay qualification is still pending on both platforms; Windows remains experimental. See [validation evidence](STATUS.md) for the distinction between automated tests, builds, and real gameplay.

## What the project provides

- one player-oriented application for setup, start, stop, configuration, and diagnostics;
- a three-step setup assistant with a pre-download readiness check, plus direct realm shortcuts on the home screen;
- a local AzerothCore authentication and world server with MySQL;
- autonomous Playerbots with separate population and proximity controls, plus a controllable companion party in game;
- managed OpenWoW on Apple Silicon macOS, plus `Wow.exe` or OpenWoW on Windows x64;
- optional, rate-limited local dialogue with direct, immersive, and lively conversation modes powered by a RealmBox-managed Ollama runtime;
- three reversible solo-progression profiles with an exact preview, durable recovery, and no character-data rewrite;
- an explicit local quest/item lookup that reads only the existing world catalogue, without AI or an external service;
- atomic installation, immutable server images, persistent character data, and complete verified backups, either automatic before migrations or created on demand.

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

- Apple Silicon Mac or Windows x64 PC (experimental);
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

## Bot experience

RealmBox does not infer one bot choice from another. The world dimension contains two separate controls—population and presence—and remains independent from in-game party behavior and local dialogue.

| Control | Choices | Effect |
| --- | --- | --- |
| Population | **5**, **25**, **50**, **100**, or **150** | Sets the requested number of autonomous adventurers; RealmBox still applies the safe Docker-memory ceiling. |
| Presence | **Dispersed**, **Natural** (recommended), or **Always nearby** | Chooses whether native Playerbots travel stays in charge, a smaller passing population visits the player's area, or the policy requests a denser nearby target. |
| In-game party | **Escort**, **Guard**, or **Free** | Applies only to the four companions controlled through the addon: follow, hold position, or resume autonomous activities. |
| Conversation | **Direct**, **Immersive**, or **Lively** | Allows only player-directed replies, occasional contextual exchanges, or more frequent but still bounded conversations. |

Fresh installations start with **Natural** presence. An installation older than 0.4.0 with no saved presence choice starts with **Always nearby**, preserving its previous dense behavior until the player changes it. Population and presence can be applied while the managed worldserver is running; otherwise they are saved for the next game.

Eligible player messages have a configured 100% reply chance, are placed ahead of queued ambient work, and have one queue slot that ambient chatter cannot occupy. This reduces starvation but does not guarantee a reply: a queue already full of player requests, a local-model failure, or a destination that disappears can still prevent delivery. Ambient Party and Raid budgets are isolated per group, with a global cap shared across ambient exchanges. Eligible player requests bypass this ambient governor, but not the queue, model, or destination checks.

None of the three modes keeps conversation history, evolving memory or relationships, and RealmBox enables neither RAG nor generated emotes. For a direct reply, the prompt asks the model to answer in the language of the latest player message. Ambient dialogue uses French prompts for a `frFR` client copy and English prompts for the other supported locales; this automated selection has not yet been qualified in OpenWoW.

The current source tree also adds named five-player squad presets, an observed primary companion, explicit party/target scope, and a preview of the exact bounded command. It never removes a group member. Remembered names are observations, not proof of bot identity, so RealmBox deliberately does not promise to recall the same bots until Playerbots provides an atomic typed server contract. See [the addon contract](docs/COMPANION_ADDON.md).

## Solo progression and the Local guide

| Profile | Experience and reputation | Money drops | Primary professions |
| --- | --- | --- | --- |
| Normal | ×1 | ×1 | 2 |
| Comfortable | ×2 | ×1 | 11 |
| Accelerated | ×3 | ×2 | 11 |

Comfortable and Accelerated also relax instance level and raid-group requirements and allow normal quests in raids. Apply changes with the world stopped, after reviewing the exact values. Restoring the previous rules does not remove rewards or professions already gained. Enemy difficulty does not change.

The Local guide searches quest or item names in the existing world catalogue. It reads no inventory or quest log and provides no personalised progression advice. Results identify their source and whether information is partial or unavailable.

These features are included in the 0.5.0 source; their full gameplay path still needs qualification. See [exact values, recovery, and limits](docs/SOLO_PROFILES_AND_LOCAL_GUIDE.md).

## Installation

1. Install and start [Docker Desktop](https://www.docker.com/products/docker-desktop/).
2. Download the compatible WoW data from the [French](https://chromiecraft.com/fr/telechargements/) or [English](https://chromiecraft.com/en/downloads/) ChromieCraft page.
3. Check the [0.5.0 preview assets](https://github.com/bnjdpn/RealmBox/releases/tag/v0.5.0). Once the installer for your platform and `SHA256SUMS.txt` are available, download them and compare the checksum.
4. In **Your copy of WoW**, select the game folder or `Data`; download help is available in the same view.
5. Choose **Your companions**, then review **Your installation**. Resolve any readiness warning before selecting **Install**.
6. Select **Play** when RealmBox reports that Azeroth is ready.

Distribution signing and notarization are not provided. Do not bypass an operating-system warning unless the downloaded artifact's SHA-256 matches the published checksum. See the [complete installation guide](docs/INSTALLATION.md) for platform details.

These steps describe the 0.5.0 setup assistant. Check the availability note above before downloading. See [the UX and safety contract](docs/SETUP_EXPERIENCE.md).

## Persistence and update safety

Release installers wait for freshly built server images from the exact release commit and embed their immutable digests. They do not reuse repository-level image variables. See [build and release provenance](docs/BUILDING.md).

The Docker Compose project name is permanently `realmbox-v3`; it is not an application version. Keeping it stable preserves the player database volume across application releases.

Before the first migration performed by each desktop version, RealmBox:

1. exports all expected MySQL databases with a consistent dump;
2. verifies the dump contents and SHA-256;
3. stores the backup outside the replaceable runtime without overwriting an existing backup;
4. applies the migration;
5. advances the migrated-version marker only after success.

After installation, **Settings → Protection** can also create a new complete, verified restore point on demand. If the world is open, RealmBox takes the consistent backup without stopping it. If it is closed, RealmBox starts only the database and stops it again afterwards. These restore points stay outside the replaceable runtime, are never overwritten, and can be used for recovery after a Docker purge.

An unknown installation schema, an incomplete backup, or a failed migration stops the update. The launcher never turns an existing realm into a fresh installation and never invokes `docker compose down --volumes` or `-v`.

If Docker Desktop is purged outside RealmBox, the launcher detects the missing volumes. The next **Play** action downloads the immutable images again, rebuilds the server resources, and restores the newest complete, verified player backup before migrations. Without a valid backup, it stops instead of silently creating an empty realm.

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
pnpm test:guide-sql # isolated MySQL proof; never uses the player's database
pnpm site:build
cargo test --workspace
cargo xtask release check
```

## Technical documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Installation](docs/INSTALLATION.md)
- [Guided setup and home screen](docs/SETUP_EXPERIENCE.md)
- [Changelog](CHANGELOG.md) and [validation status](STATUS.md)
- [Compatibility](docs/COMPATIBILITY.md)
- [Updates and backups](docs/UPDATES.md)
- [Security](docs/SECURITY.md)
- [Building](docs/BUILDING.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Playerbots population, presence, and party behavior](docs/PLAYERBOTS_INTEGRATION.md)
- [Local dialogue](docs/OLLAMA_CHAT_INTEGRATION.md)
- [Solo profiles and local guide](docs/SOLO_PROFILES_AND_LOCAL_GUIDE.md)
- [Companion addon and bounded commands](docs/COMPANION_ADDON.md)
- [Review of the solo and bot projects studied](docs/ECOSYSTEM_REVIEW_2026-09-03.md)
- [Distribution and licenses](docs/LEGAL_AND_DISTRIBUTION.md)

## License and independence

RealmBox is distributed under [AGPL-3.0-only](LICENSE). It is an independent project and is not affiliated with or endorsed by Blizzard Entertainment or ChromieCraft. World of Warcraft, Azeroth, and related names belong to their respective owners.
