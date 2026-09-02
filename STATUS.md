# État factuel

Mis à jour le 2 septembre 2026 sur macOS 26.6.2 arm64.

| Fonction | macOS arm64 | macOS x86-64 | Windows x64 | Preuve |
|---|---|---|---|---|
| UI RealmBox | testé + QA visuelle | non commencé | non commencé | Vitest, build Vite, Playwright 1200 px et 390 px |
| Machine à états | testé automatiquement | testé automatiquement | testé automatiquement | tests Rust indépendants plateforme |
| SQLite/migrations | testé automatiquement | testé automatiquement | testé automatiquement | tests mémoire + chemin Unicode |
| Faux onboarding complet | testé avec fake | non commencé | workflow créé | tests frontend |
| Faux Jouer/arrêt/conversation | testé avec fake | non commencé | workflow créé | tests frontend et orchestrateur |
| Bundle Tauri | buildé | non commencé | workflow créé | `RealmBox.app` local arm64 sans signature de distribution |
| OpenWoW | buildé | workflow créé | workflow créé | pin `2521e1f`; Mach-O arm64 55 Mio, SHA-256 consigné |
| Données OpenWoW réelles | bloqué | bloqué | bloqué | données utilisateur absentes |
| Serveur Playerbots | buildé | workflow créé | workflow créé | `authserver` et `worldserver` arm64 produits; exécution non prouvée |
| mod-ollama-chat | compilé en bibliothèque statique | non commencé | non commencé | module inclus dans `worldserver`; requête Ollama non prouvée |
| MariaDB locale | spike connecteur en échec | non commencé | non commencé | Connector/C 3.4.9 incompatible avec les API MySQL 8 attendues; runtime non sélectionné |
| Ollama | bloqué | bloqué | bloqué | absent de la machine; runtime non audité |
| CanIRun | bloqué | bloqué | bloqué | aucune API publique officielle identifiée |
| Addon Companions | implémenté | implémenté | implémenté | squelette Lua; test en jeu non exécuté |
| Parcours réel complet | bloqué | bloqué | bloqué | aucun personnage, groupe ou dialogue réel testé |
| Signature/notarisation | bloqué | bloqué | bloqué | certificats absents |

Les statuts de build seront mis à jour uniquement après lecture des artefacts produits. Un workflow présent n'est pas une preuve qu'il passe.
