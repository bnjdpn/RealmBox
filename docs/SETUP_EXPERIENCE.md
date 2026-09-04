# Installation guidée et accueil / Guided setup and home

Source non publiée après 0.4.0, 4 septembre 2026. Ce document ne décrit pas un nouveau bundle installé ou distribué.

Unreleased source after 0.4.0, 4 September 2026. This document does not describe a newly installed or distributed bundle.

## Français

Le parcours normal conserve une seule fenêtre 1024 × 640. Les actions de navigation restent visibles pendant que le contenu long défile à l’intérieur du panneau.

1. **Votre copie de WoW** : choisir la racine du jeu ou `Data`. La sélection est inspectée avant de permettre la suite ; l’annulation conserve le dernier dossier valide. Un lien d’aide ouvre la page ChromieCraft de la langue choisie, sans télécharger de données de jeu dans RealmBox. Les archives reconnues ne sont pas présentées comme une preuve de build exacte : celle-ci est confirmée par les extracteurs. Le client original est proposé uniquement sur Windows.
2. **Vos compagnons** : trois cartes de population (25, 50, 100) et un choix personnalisé (5/25/50/100/150), présence indépendante et dialogues locaux facultatifs. Aucun modèle n’est téléchargé avant confirmation. Le modèle, sa taille et sa licence sont présentés si l’option est choisie ; un modèle indisponible ou un manque d’espace bloque cette option.
3. **Votre installation** : récapitulatif du dossier, du client, des bots, de la présence et du dialogue ; vérification de la plateforme, de la destination, de Docker/Compose et de l’espace disque. Le bouton **Installer** reste indisponible pendant une vérification, après un échec ou en présence d’un contrôle bloquant. Les populations souhaitée et prévue avec la mémoire détectée restent distinctes.

Les retours en arrière ne perdent pas les choix. Un changement de modèle invalide une mesure disque précédente, même si sa réponse arrive en retard. Après échec d’installation, **Réessayer** relit l’état existant, conserve le brouillon tant qu’aucun royaume n’est installé et ne réinstalle jamais automatiquement. Ces choix de première installation sont conservés dans la session, pas dans un nouveau fichier de préférences.

L’accueil d’un royaume installé garde **Jouer / Arrêter le monde** comme action principale et donne accès directement à la population/présence, aux dialogues, au rythme solo, à la protection et au guide. La population affichée vient de la configuration appliquée, pas d’un comptage des bots connectés. Une aide dépliable explique la connexion au compte local. Les erreurs techniques et journaux restent dans **Diagnostic**.

## English

The normal flow stays in a single 1024 × 640 window, with persistent navigation actions and an internally scrolling panel.

1. **Your copy of WoW**: inspect the game root or `Data` before proceeding. Cancelling keeps the last valid selection. Download help opens the selected-language ChromieCraft page; RealmBox does not download proprietary game data. Recognized archives are not proof of the exact build, which is confirmed during extraction. The original client option is Windows-only.
2. **Your companions**: three population cards (25, 50, 100), a custom choice (5/25/50/100/150), independent presence, and optional local dialogue. No model download before confirmation. The chosen model, size and license are shown; an unavailable model or insufficient space disables the option.
3. **Your installation**: review folder, client, bots, presence and dialogue; check platform, destination, Docker/Compose and free space. **Install** stays disabled while checks are pending, failed or blocking. Requested population and planned memory-limited population are separate.

Back navigation preserves choices. Changing the optional model invalidates an older disk check, including a late result. After installation failure, **Try again** reads back state without automatically reinstalling. Setup choices survive within the current session; they do not introduce a new persistent preference file.

The installed home keeps **Play / Stop the world** primary, with shortcuts to population/presence, dialogue, solo pace, protection and local lookup. Displayed population is the applied configuration, not online telemetry. Expandable sign-in help shows the local account. Technical detail remains in **Diagnostics**.

## Safety and evidence boundaries

- `inspect_installation` is read-only: bounded `docker info` and `docker compose version` (10 seconds each), filesystem free-space measurement and target/platform inspection. It never starts Docker, pulls an image, creates a container or accesses a database. Its small process-capture files use the existing private temporary-file mechanism.
- Base disk requirement is shared with the installer (24 GiB plus the allowlisted optional model). Unknown disk measurement fails closed. Unknown Docker memory is shown as unknown; the existing installer retains its conservative bot ceiling.
- A destination with an installation manifest, runtime, dangling link or unreadable filesystem state is not fresh. The installer keeps its independent checks; preflight is not authority to overwrite an existing realm.
- Help commands accept only `gameFr`, `gameEn` or `docker`, mapped to fixed HTTPS destinations. No arbitrary URL or command reaches the bridge. Opening a browser is an explicit OS handoff, not a managed server process that RealmBox may terminate.
- Pure/temporary-file Rust tests, mocked React transitions and browser fixtures are distinct from native macOS/Windows verification. Browser `previewState=setup` is development-only simulated data; production browser mode cannot attest to local readiness.
- No real world installation, database migration, bundle replacement, Docker startup or player-data change was performed for this UX work. Native folder picker, external browser handoff and end-to-end installation still need validation on each platform.

## Inspiration

The public [WOW Legends app page](https://wow-legends.eu/app) presents installation, realm control, settings and backups in one window. Its [public dashboard screenshot](https://wow-legends.eu/assets/img/screenshots/app_dashboard.webp), inspected on 4 September 2026, shows a central realm action, grouped navigation, summary cards and a setup checklist; some entries in that image are marked “soon”. That is public design evidence, not a test of the current supporter-only application.

RealmBox adapts the grouped journey, visible next action and contextual shortcuts, using its own existing artwork. It does not copy assets or add hosted AI, a repack, account administration, remote access or unverified live statistics.
