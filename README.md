# RealmBox

RealmBox est un lanceur macOS pour jouer sur un monde 3.3.5a entièrement local. Au premier lancement, il demande le dossier d'une copie compatible appartenant au joueur, puis prépare automatiquement le client ouvert, le serveur, la base locale et Playerbots. Si la machine est assez confortable, il peut aussi installer Ollama et `mod-ollama-chat` pour donner aux compagnons des dialogues calculés localement. Aux lancements suivants, il démarre la pile dans l'ordre et ouvre directement le client.

L'interface reprend la composition des lanceurs MMO de l'ère Wrath — grande scène originale, nouvelles à droite, métal bleuté, barre de mise à jour et grand bouton d'action — sans logo, illustration, texte ni ressource Blizzard.

## Premier lancement

1. Démarrer Docker Desktop.
2. Ouvrir RealmBox et choisir le dossier qui contient `Data` dans une copie 3.3.5a build 12340 obtenue légalement.
3. Activer ou désactiver les compagnons Playerbots.
4. Facultatif : accepter les dialogues IA si CanIRun classe un modèle compact comme confortable sur cette machine.
5. Cliquer sur **Installer**.

RealmBox télécharge l'artefact officiel OpenWoW 0.1.2 et vérifie son SHA-256, récupère les commits immuables du serveur et de Playerbots, construit les images Docker, extrait `maps`, `vmaps`, `mmaps` et `dbc` depuis les données locales, initialise MySQL et crée le compte de joueur local `REALMBOX / REALMBOX`. Pour l'option dialogue, il récupère aussi le commit épinglé de `mod-ollama-chat`, l'archive Ollama 0.33.2 vérifiée et le modèle autorisé choisi avec CanIRun.

La requête CanIRun contient uniquement le nom du processeur, le nombre de cœurs et la quantité de mémoire. RealmBox teste une liste fermée de petits modèles et réserve au maximum 25 % de la RAM (plafond 8 Go) afin de laisser fonctionner le client, le serveur et les bots. CanIRun donne une estimation, pas un benchmark. Si le service est indisponible ou si aucun modèle n'est confortable, seuls les dialogues LLM sont désactivés ; Playerbots continue de fonctionner.

RealmBox ne télécharge, ne copie dans le dépôt et ne distribue aucune donnée propriétaire. Le dossier `Data` reste à son emplacement d'origine et n'est monté qu'en lecture seule pendant l'extraction.

## Lancements suivants

L'ouverture de RealmBox déclenche automatiquement :

```text
MySQL local → vérification des données serveur → migrations → Ollama local si demandé → authserver/worldserver → OpenWoW
```

Playerbots est activé uniquement si le joueur l'a demandé. Ollama écoute sur `127.0.0.1:11435`, avec les fonctions cloud désactivées pendant le jeu, une seule requête parallèle et un seul modèle chargé. Les ports du serveur et de la base sont également liés à `127.0.0.1`. RealmBox surveille le processus OpenWoW et arrête automatiquement le monde, la base et Ollama quand le client se ferme. Les journaux et l'installation gérée restent dans le répertoire applicatif `org.realmbox.desktop`.

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
