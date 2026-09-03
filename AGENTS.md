# RealmBox contributor instructions

- Never add proprietary game data, extracted server data, credentials, models, or user databases.
- Never delete, recreate, rename, or detach the persistent player database volume. Keep the Compose project name `realmbox-v3` stable across app releases and never use `docker compose down --volumes` or `-v`.
- Every database/schema migration must fail closed behind a complete, verified, non-overwriting pre-migration backup stored outside the replaceable runtime. Never treat an unknown installation schema as a fresh install.
- Every distributed update must increment the desktop Cargo package version; this version is the durable trigger for the mandatory pre-migration backup.
- Never implement an update by calling the first-install path over an existing realm. Runtime replacement must be staged atomically and must preserve the player-data backup and rollback contract in `docs/UPDATES.md`.
- Keep the normal product flow player-oriented; server details belong only in diagnostics.
- Keep platform and runtime effects behind typed interfaces and test them with fakes on macOS.
- Distinguish fake, automated, build, manual, and full real-path evidence in `STATUS.md`.
- Pin every production upstream to an immutable commit before building or distributing it.
- Add tests for state transitions, configuration changes, security boundaries, and recovery.
- Run `pnpm verify` before a milestone commit when the toolchain permits it.
