# RealmBox contributor instructions

- Never add proprietary game data, extracted server data, credentials, models, or user databases.
- Keep the normal product flow player-oriented; server details belong only in diagnostics.
- Keep platform and runtime effects behind typed interfaces and test them with fakes on macOS.
- Distinguish fake, automated, build, manual, and full real-path evidence in `STATUS.md`.
- Pin every production upstream to an immutable commit before building or distributing it.
- Add tests for state transitions, configuration changes, security boundaries, and recovery.
- Run `pnpm verify` before a milestone commit when the toolchain permits it.

