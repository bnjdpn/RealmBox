# RealmBox

[Français](README.md) · [English](README.en.md)

[![Validation commune](https://github.com/bnjdpn/RealmBox/actions/workflows/validation.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/validation.yml)
[![macOS arm64](https://github.com/bnjdpn/RealmBox/actions/workflows/macos-arm64.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/macos-arm64.yml)
[![Windows x64](https://github.com/bnjdpn/RealmBox/actions/workflows/windows-x64.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/windows-x64.yml)
[![Site RealmBox](https://github.com/bnjdpn/RealmBox/actions/workflows/pages.yml/badge.svg)](https://bnjdpn.github.io/RealmBox/?lang=fr)
[![Licence AGPL-3.0](https://img.shields.io/badge/licence-AGPL--3.0-blue.svg)](LICENSE)

RealmBox est un lanceur Windows x64 et macOS Apple Silicon pour jouer sur un monde 3.3.5a entièrement local. Au premier lancement, il demande le dossier d'une copie compatible appartenant au joueur, puis prépare le client, le serveur, la base locale et Playerbots. Si la machine est assez confortable, il peut aussi installer Ollama et `mod-ollama-chat` pour calculer localement les dialogues des bots. Aux lancements suivants, il démarre la pile dans l'ordre et ouvre le client.

L’interface 0.3.0 est disponible en français et en anglais. Elle sépare **Mon monde**, **Compagnons**, **Dialogues** et **Diagnostic**, montre une cause courte avec l’action de récupération utile et garde les détails serveur hors du parcours joueur.

Le launcher et le site partagent une identité originale de portail fantasy nordique : deux panoramas raster distincts créés pour RealmBox avec ImageGen et un médaillon R/B. Le launcher est une scène fixe de 1024 × 640 centrée sur l’état courant et une seule action ; langue, compagnons, dialogues et diagnostic restent dans les réglages. Aucun asset de jeu ni police distante n’est utilisé. Le [design system](docs/DESIGN_SYSTEM.md), l’[audit de réinterprétation historique](docs/WOTLK_ERA_VISUAL_AUDIT.md) et la [provenance de chaque asset](docs/ASSET_PROVENANCE.md) sont documentés dans le dépôt.

## Télécharger et suivre le tutoriel

- [Site RealmBox et tutoriel FR](https://bnjdpn.github.io/RealmBox/?lang=fr)
- [Préversions macOS arm64 et Windows x64](https://github.com/bnjdpn/RealmBox/releases)
- [État factuel et limites de preuve](STATUS.md)

Les artefacts actuels sont des préversions non signées. Vérifiez toujours le fichier `SHA256SUMS.txt` joint à la release. Le parcours macOS Apple Silicon a été qualifié localement ; l’installateur Windows x64 est construit en CI mais le parcours complet Windows 11 reste à tester.

## Premier lancement

1. Démarrer Docker Desktop.
2. Choisir le client : **OpenWoW géré par RealmBox** (recommandé) ou, sous Windows x64, **client original fourni par le joueur**.
3. Choisir le dossier qui contient `Data` dans une copie 3.3.5a build 12340 obtenue légalement. RealmBox contrôle immédiatement les signatures MPQ, les archives WotLK (`common`, `expansion`, `lichking`) et l’archive de locale avant d’autoriser l’installation.
4. Activer ou désactiver Playerbots et choisir une population de 5, 25, 50, 100 ou 150 bots. RealmBox limite la valeur selon la mémoire réellement accordée à Docker.
5. Facultatif : cocher les dialogues locaux après lecture du modèle, de la taille et de la vitesse que RealmBox a décidés avec CanIRun. Le modèle n’est jamais choisi manuellement.
6. Cliquer sur **Installer**.

Avec le choix recommandé, RealmBox télécharge lui-même l'artefact officiel OpenWoW 0.1.2 correspondant à la plateforme et vérifie son SHA-256 avant extraction. Avec le choix avancé Windows, il exige `Wow.exe`, ne télécharge aucun client propriétaire, sauvegarde le `realmlist.wtf` existant puis configure uniquement la connexion locale. Le lanceur consomme quatre images RealmBox multiarchitecture précompilées et épinglées par digest : le Compose joueur ne contient aucun build C++ et télécharge uniquement les images correspondant à la machine. Les manifestes linux-amd64 et linux-arm64 ont été construits depuis les commits immuables, puis téléchargés sans authentification dans GitHub Actions. Dans les deux cas, RealmBox extrait `maps`, `vmaps`, `mmaps` et `dbc` depuis les données locales, initialise MySQL et crée le compte local `REALMBOX / REALMBOX`. Pour l'option dialogue, il télécharge l'archive Ollama 0.33.2 vérifiée puis refuse le modèle si son manifeste ne correspond pas au digest immuable du catalogue RealmBox.

La requête CanIRun contient uniquement le nom du processeur, le nombre de cœurs et la quantité de mémoire. RealmBox teste une liste fermée de petits modèles, conserve ceux classés confortables dans un budget maximal de 25 % de la RAM (plafond 8 Go), puis choisit automatiquement le meilleur rapport entre vitesse estimée et taille officielle de téléchargement. Un modèle 1B sert uniquement de repli quand aucun modèle 3B ou supérieur n’est confortable. CanIRun donne une estimation, pas un benchmark. Si le service est indisponible ou si aucun modèle n'est confortable, seuls les dialogues LLM sont désactivés ; Playerbots continue de fonctionner.

Après l’installation initiale, la vue **Dialogues** permet la même activation sur demande sans réinstaller le royaume. Le téléchargement ne commence qu’après confirmation, et la désactivation conserve le modèle local pour une réactivation rapide. Le monde doit être fermé pour que `worldserver` recharge cette configuration.

RealmBox ne télécharge, ne copie dans le dépôt et ne distribue aucune donnée propriétaire. Le dossier `Data` reste à son emplacement d'origine et n'est monté qu'en lecture seule pendant l'extraction.

Après l’installation, **Réglages → Client de jeu** affiche le dossier utilisé et permet d’en choisir un autre lorsque le monde est arrêté. RealmBox valide de nouveau les archives 3.3.5a, reconstruit atomiquement les liens de données d’OpenWoW ou met à jour le chemin de `Wow.exe`, sans réinstaller le serveur ni toucher à la base des personnages.

## Mises à jour sans perte de personnages

RealmBox refuse toute réinstallation par-dessus un royaume existant et interdit à ses commandes Docker de supprimer les volumes persistants. Avant la première migration de chaque version, il exporte automatiquement les quatre bases locales, vérifie que le dump contient notamment les comptes et personnages, puis écrit un SHA-256. La sauvegarde reste hors du runtime dans `player-data-backups` et n’est jamais écrasée. Si cette preuve échoue, la migration et le démarrage sont annulés sans modifier le marqueur de version. Voir le [contrat de mise à jour](docs/UPDATES.md).

## Lancements suivants

Quand le joueur choisit **Jouer**, RealmBox déclenche :

```text
MySQL local → sauvegarde si nouvelle version → vérification des données serveur → migrations → Ollama local si demandé → authserver/worldserver → client choisi
```

Playerbots est activé uniquement si le joueur l'a demandé. Ollama écoute sur `127.0.0.1:11435`, avec les fonctions cloud désactivées pendant le jeu, une seule requête parallèle et un seul modèle chargé. Les ports de jeu sont liés à `127.0.0.1` et MySQL n'est pas publié sur l'hôte. RealmBox surveille le processus client qu'il a démarré et arrête automatiquement le monde, la base et Ollama quand ce client se ferme. Les journaux, sauvegardes de configuration et composants gérés restent dans le répertoire applicatif `org.realmbox.desktop`.

Dans le jeu, l’addon RealmBox permet de former une équipe équilibrée de quatre bots au niveau du joueur, puis de leur demander de suivre, attaquer, attendre ou se regrouper. Son panneau FR/EN se réduit dans une icône déplaçable autour de la minimap ; son ouverture, la position de l’icône et celle du panneau sont conservées. `/realmbox` ou `/rb` permet aussi de l’afficher et le masquer. Le panneau indique la composition réellement visible du groupe, désactive les ordres impossibles et pilote explicitement la stratégie Playerbots des capacités fortes. Les autres bots continuent de parcourir le monde de façon autonome.

Pendant la partie, la vue **Compagnons** peut modifier la population demandée sans fermer le client. RealmBox recalcule d’abord la limite mémoire, recharge `playerbots.conf` avec la commande Playerbots officielle puis déclenche une mise à jour complète. La connexion ou déconnexion effective des bots peut prendre quelques instants.

## Client selon la plateforme

- **Windows x64** : une copie 3.3.5a avec `Wow.exe` peut être lancée directement ; OpenWoW reste disponible comme option expérimentale.
- **macOS Apple Silicon** : le package Windows fournit les données, mais pas un exécutable natif. RealmBox utilise OpenWoW arm64 pour éviter une machine virtuelle Windows.
- **Linux** : prévu, non implémenté. Les options à qualifier sont OpenWoW natif et le client Windows via Wine.

La matrice et les décisions détaillées sont suivies dans [ROADMAP.md](ROADMAP.md) et [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md).

## Développement

Le développement local observé reste macOS Apple Silicon. Le produit cible aussi Windows x64 ; le chemin Windows compile avec le toolchain MSVC et dispose d'une CI dédiée, mais doit encore être exécuté sur une vraie machine Windows. Le parcours joueur requiert actuellement Docker Desktop démarré, mais plus Git ni curl : RealmBox télécharge par son client HTTP intégré et utilise les images serveur immuables. Node, pnpm, Git et Rust ne sont requis que pour construire RealmBox ou exécuter le mode développeur sans images précompilées.

```sh
pnpm install
pnpm dev          # application Tauri, commandes réelles
pnpm dev:preview  # aperçu navigateur, aucune simulation d'installation
pnpm verify
cargo xtask release check  # cohérence versions, manifeste et statuts de plateforme
python3 -m http.server 1421 --directory site  # aperçu du site Pages
```

Le parcours complet avec données de jeu n'est pas déclaré validé tant qu'une copie utilisateur n'a pas permis de terminer l'installation et d'entrer en jeu. Voir [STATUS.md](STATUS.md) pour la séparation entre tests, build et preuve réelle.

RealmBox est licencié sous AGPL-3.0-only. Les contributions sont décrites dans [CONTRIBUTING.md](CONTRIBUTING.md). La redistribution de binaires et de leurs dépendances doit encore faire l'objet d'une revue juridique avant publication ; aucune donnée WoW n'est acceptée dans les issues, artefacts ou releases.
