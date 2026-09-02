# Spikes upstream du 2 septembre 2026

Environnement observé: macOS 26.6.2 arm64, Apple Clang 17, CMake 4.4.3,
Rust 1.97.1 et Node 25.2.1. Les sources ont été clonées hors du dépôt, dans un
répertoire temporaire, sans aucune donnée de jeu.

## OpenWoW

- Révision: `2521e1f72eeffd661913be63d1e2b374073c316c`.
- Baseline vcpkg imposée par le dépôt: `74a896d035ae4a333e7404d510aede88acea4e41`.
- `cmake --preset ci-macos-arm64`: réussi.
- `cmake --build --preset ci-macos-arm64 --target openwow-client --parallel 8`:
  réussi; Mach-O arm64 de 55 Mio, signature ad hoc.
- SHA-256 du binaire observé:
  `b5c1c777263c831f3b42fb683d0cbf783c6c3bcf45fde08ea97a1ca50e380cfe`.
- Aucun lancement réel n'est possible sans données utilisateur compatibles.

## Serveur et modules

- Core: `47960183bb03b83e8943eb2f0f39c16df9710c9d`.
- Playerbots: `2f7d9f774987d0157c6a0d0cc08c40bec3db3945`.
- Ollama chat: `a9d14b0b8955be136e657ac168dd255f5281a535`.
- Les deux modules sont détectés par CMake à ces révisions.
- MariaDB Connector/C 3.4.9 configure le projet mais ne compile pas le core:
  les symboles MySQL 8 `mysql_ssl_mode`, `MYSQL_OPT_SSL_MODE` et
  `mysql_stmt_bind_named_param` sont absents.
- Reconfiguration avec MySQL Client 26.7: réussie. Le premier build atteint
  99 %, puis `worldserver` échoue car CMake choisit le faux `libreadline` du
  SDK macOS, dont les en-têtes n'exposent pas `rl_done` et `rl_event_hook`.
- Une seconde reconfiguration épingle donc explicitement GNU Readline
  Homebrew: build complet réussi.
- Artefacts natifs arm64: `authserver` (1,9 Mio), `worldserver` (49 Mio) et
  `dbimport`. SHA-256 de `worldserver`:
  `51c6137ddb027b663418c60c59b19149f68fb796fc35ffb9ee3b0f0c5682e7ae`.
- `worldserver` reste lié dynamiquement aux bibliothèques Homebrew MySQL,
  Readline, OpenSSL et Boost. Ce n'est donc pas encore un runtime portable.

Ce spike prouve uniquement la configuration et la compilation observées. Il ne
prouve ni import de données, ni démarrage de la base, ni connexion client, ni
session de jeu.
