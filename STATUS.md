# État factuel

Mis à jour le 2 septembre 2026 sur macOS 26.6.2 arm64.

| Fonction | macOS arm64 | macOS x86-64 | Windows x64 | Preuve |
|---|---|---|---|---|
| UI RealmBox | testé automatiquement | non commencé | non commencé | Vitest + build Vite |
| Machine à états | testé automatiquement | testé automatiquement | testé automatiquement | tests Rust indépendants plateforme |
| SQLite/migrations | testé automatiquement | testé automatiquement | testé automatiquement | tests mémoire + chemin Unicode |
| Faux onboarding complet | testé avec fake | non commencé | workflow créé | tests frontend |
| Faux Jouer/arrêt/conversation | testé avec fake | non commencé | workflow créé | tests frontend et orchestrateur |
| Bundle Tauri | buildé | non commencé | workflow créé | `RealmBox.app` local arm64 sans signature de distribution |
| OpenWoW | build en cours | workflow créé | workflow créé | pin `2521e1f`; configuration native séparée |
| Données OpenWoW réelles | bloqué | bloqué | bloqué | données utilisateur absentes |
| Serveur Playerbots | build en cours | workflow créé | workflow créé | core configuré avec MySQL Client 26.7; modules compilés en cours |
| mod-ollama-chat | compile dans le spike | non commencé | non commencé | module détecté et sources compilées; exécution non prouvée |
| MariaDB locale | spike connecteur en échec | non commencé | non commencé | Connector/C 3.4.9 incompatible avec les API MySQL 8 attendues; runtime non sélectionné |
| Ollama | bloqué | bloqué | bloqué | absent de la machine; runtime non audité |
| CanIRun | bloqué | bloqué | bloqué | aucune API publique officielle identifiée |
| Addon Companions | implémenté | implémenté | implémenté | squelette Lua; test en jeu non exécuté |
| Parcours réel complet | bloqué | bloqué | bloqué | aucun personnage, groupe ou dialogue réel testé |
| Signature/notarisation | bloqué | bloqué | bloqué | certificats absents |

Les statuts de build seront mis à jour uniquement après lecture des artefacts produits. Un workflow présent n'est pas une preuve qu'il passe.
