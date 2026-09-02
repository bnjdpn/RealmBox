# RealmBox

RealmBox est un lanceur macOS pour jouer sur un monde 3.3.5a entièrement local. Au premier lancement, il demande le dossier d'une copie compatible appartenant au joueur, puis prépare automatiquement le client ouvert, le serveur, la base locale et Playerbots. Aux lancements suivants, il démarre la pile dans l'ordre et ouvre directement le client.

L'interface reprend la grammaire des lanceurs MMO de l'ère Wrath — fenêtre encadrée, métal bleuté, file de mise à jour, grand bouton d'action — sans logo, illustration, texte ni ressource Blizzard.

## Premier lancement

1. Démarrer Docker Desktop.
2. Ouvrir RealmBox et choisir le dossier qui contient `Data` dans une copie 3.3.5a build 12340 obtenue légalement.
3. Activer ou désactiver les compagnons Playerbots.
4. Cliquer sur **Installer**.

RealmBox télécharge l'artefact officiel OpenWoW 0.1.2 et vérifie son SHA-256, récupère les commits immuables du serveur et de Playerbots, construit les images Docker, extrait `maps`, `vmaps`, `mmaps` et `dbc` depuis les données locales, initialise MySQL et crée le compte de joueur local `REALMBOX / REALMBOX`.

RealmBox ne télécharge, ne copie dans le dépôt et ne distribue aucune donnée propriétaire. Le dossier `Data` reste à son emplacement d'origine et n'est monté qu'en lecture seule pendant l'extraction.

## Lancements suivants

L'ouverture de RealmBox déclenche automatiquement :

```text
MySQL local → vérification des données serveur → migrations → authserver/worldserver → OpenWoW
```

Playerbots est activé uniquement si le joueur l'a demandé. Les ports du serveur et de la base sont liés à `127.0.0.1`. Les journaux et l'installation gérée restent dans le répertoire applicatif `org.realmbox.desktop`.

## Développement macOS

La première cible réelle est macOS Apple Silicon. Elle requiert Docker Desktop démarré, Git, curl, OpenSSL, Node, pnpm et Rust.

```sh
pnpm install
pnpm dev          # application Tauri, commandes réelles
pnpm dev:preview  # aperçu navigateur, aucune simulation d'installation
pnpm verify
```

Le parcours complet avec données de jeu n'est pas déclaré validé tant qu'une copie utilisateur n'a pas permis de terminer l'installation et d'entrer en jeu. Voir [STATUS.md](STATUS.md) pour la séparation entre tests, build et preuve réelle.

RealmBox est licencié sous AGPL-3.0-only. La redistribution de binaires et de leurs dépendances doit encore faire l'objet d'une revue juridique avant publication.
