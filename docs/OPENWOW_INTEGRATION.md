# Intégration OpenWoW

Source épinglée : `rkabachenko/OpenWow-snapshot` commit `2521e1f72eeffd661913be63d1e2b374073c316c`, version source 0.1.2, AGPL-3.0-only. Le dépôt public est un snapshot régénéré, avec historique limité. Il annonce C++20, CMake 3.24+, Ninja et vcpkg épinglé au baseline `74a896d035ae4a333e7404d510aede88acea4e41`.

Les presets upstream incluent macOS arm64 natif, macOS x86-64 cross-compilé sur arm64 et Windows x64 statique. Le README annonce connexion, royaume, personnage, monde, mouvement, combat, chat, inventaire, quêtes et addons mais qualifie le projet d'expérimental; RealmBox considère donc chaque capacité non validée jusqu'au smoke test.

Le premier essai de configuration a échoué car le preset CI nécessite explicitement `VCPKG_ROOT`; il a été relancé avec ce chemin. Aucun test avec données de jeu n'est possible dans le dépôt actuel.

