# RealmBox

RealmBox est un lanceur Windows x64 et macOS Apple Silicon pour jouer sur un monde 3.3.5a entièrement local. Au premier lancement, il demande le dossier d'une copie compatible appartenant au joueur, puis prépare automatiquement le client, le serveur, la base locale et Playerbots. Si la machine est assez confortable, il peut aussi installer Ollama et `mod-ollama-chat` pour donner aux compagnons des dialogues calculés localement. Aux lancements suivants, il démarre la pile dans l'ordre et ouvre directement le client.

L'interface reprend la composition des lanceurs MMO de l'ère Wrath — grande scène originale, nouvelles à droite, métal bleuté, barre de mise à jour et grand bouton d'action — sans logo, illustration, texte ni ressource Blizzard.

## Premier lancement

1. Démarrer Docker Desktop.
2. Choisir le client : **OpenWoW géré par RealmBox** (recommandé) ou, sous Windows x64, **client original fourni par le joueur**.
3. Choisir le dossier qui contient `Data` dans une copie 3.3.5a build 12340 obtenue légalement.
4. Activer ou désactiver les compagnons Playerbots.
5. Facultatif : accepter les dialogues IA si CanIRun classe un modèle compact comme confortable sur cette machine.
6. Cliquer sur **Installer**.

Avec le choix recommandé, RealmBox télécharge lui-même l'artefact officiel OpenWoW 0.1.2 correspondant à la plateforme et vérifie son SHA-256 avant extraction. Avec le choix avancé Windows, il exige `Wow.exe`, ne télécharge aucun client propriétaire, sauvegarde le `realmlist.wtf` existant puis configure uniquement la connexion locale. Dans le prototype actuel, RealmBox récupère les commits immuables du serveur et de Playerbots puis construit encore les images Docker localement afin de qualifier exactement les binaires. Une release joueur devra télécharger les images RealmBox multiarchitecture précompilées et épinglées par digest ; elle ne fera pas compiler C++ sur la machine du joueur. Dans les deux cas, RealmBox extrait `maps`, `vmaps`, `mmaps` et `dbc` depuis les données locales, initialise MySQL et crée le compte local `REALMBOX / REALMBOX`. Pour l'option dialogue, il récupère aussi le commit épinglé de `mod-ollama-chat`, l'archive Ollama 0.33.2 vérifiée et le modèle autorisé choisi avec CanIRun.

La requête CanIRun contient uniquement le nom du processeur, le nombre de cœurs et la quantité de mémoire. RealmBox teste une liste fermée de petits modèles et réserve au maximum 25 % de la RAM (plafond 8 Go) afin de laisser fonctionner le client, le serveur et les bots. CanIRun donne une estimation, pas un benchmark. Si le service est indisponible ou si aucun modèle n'est confortable, seuls les dialogues LLM sont désactivés ; Playerbots continue de fonctionner.

RealmBox ne télécharge, ne copie dans le dépôt et ne distribue aucune donnée propriétaire. Le dossier `Data` reste à son emplacement d'origine et n'est monté qu'en lecture seule pendant l'extraction.

## Lancements suivants

L'ouverture de RealmBox déclenche automatiquement :

```text
MySQL local → vérification des données serveur → migrations → Ollama local si demandé → authserver/worldserver → client choisi
```

Playerbots est activé uniquement si le joueur l'a demandé. Ollama écoute sur `127.0.0.1:11435`, avec les fonctions cloud désactivées pendant le jeu, une seule requête parallèle et un seul modèle chargé. Les ports de jeu sont liés à `127.0.0.1` et MySQL n'est pas publié sur l'hôte. RealmBox surveille le processus client qu'il a démarré et arrête automatiquement le monde, la base et Ollama quand ce client se ferme. Les journaux, sauvegardes de configuration et composants gérés restent dans le répertoire applicatif `org.realmbox.desktop`.

## Développement

Le développement local observé reste macOS Apple Silicon. Le produit cible aussi Windows x64 ; le chemin Windows compile avec le toolchain MSVC et dispose d'une CI dédiée, mais doit encore être exécuté sur une vraie machine Windows. Le parcours requiert Docker Desktop démarré, Git et curl ; Node, pnpm et Rust ne sont requis que pour construire RealmBox.

```sh
pnpm install
pnpm dev          # application Tauri, commandes réelles
pnpm dev:preview  # aperçu navigateur, aucune simulation d'installation
pnpm verify
```

Le parcours complet avec données de jeu n'est pas déclaré validé tant qu'une copie utilisateur n'a pas permis de terminer l'installation et d'entrer en jeu. Voir [STATUS.md](STATUS.md) pour la séparation entre tests, build et preuve réelle.

RealmBox est licencié sous AGPL-3.0-only. La redistribution de binaires et de leurs dépendances doit encore faire l'objet d'une revue juridique avant publication.
