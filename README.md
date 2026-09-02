# RealmBox

[Français](README.md) · [English](README.en.md)

[![Validation commune](https://github.com/bnjdpn/RealmBox/actions/workflows/validation.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/validation.yml)
[![macOS arm64](https://github.com/bnjdpn/RealmBox/actions/workflows/macos-arm64.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/macos-arm64.yml)
[![Windows x64](https://github.com/bnjdpn/RealmBox/actions/workflows/windows-x64.yml/badge.svg)](https://github.com/bnjdpn/RealmBox/actions/workflows/windows-x64.yml)
[![Site RealmBox](https://github.com/bnjdpn/RealmBox/actions/workflows/pages.yml/badge.svg)](https://bnjdpn.github.io/RealmBox/?lang=fr)
[![Licence AGPL-3.0](https://img.shields.io/badge/licence-AGPL--3.0-blue.svg)](LICENSE)

RealmBox est un lanceur Windows x64 et macOS Apple Silicon pour jouer sur un monde 3.3.5a entièrement local. Au premier lancement, il demande le dossier d'une copie compatible appartenant au joueur, puis prépare le client, le serveur, la base locale et Playerbots. Si la machine est assez confortable, il peut aussi installer Ollama et `mod-ollama-chat` pour calculer localement les dialogues des bots. Aux lancements suivants, il démarre la pile dans l'ordre et ouvre le client.

L’interface 0.2.0 est disponible en français et en anglais. Elle sépare **Mon monde**, **Compagnons** et **Diagnostic**, montre une cause courte avec l’action de récupération utile et garde les détails serveur hors du parcours joueur.

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
5. Facultatif : accepter les dialogues IA si CanIRun classe un modèle compact comme confortable sur cette machine.
6. Cliquer sur **Installer**.

Avec le choix recommandé, RealmBox télécharge lui-même l'artefact officiel OpenWoW 0.1.2 correspondant à la plateforme et vérifie son SHA-256 avant extraction. Avec le choix avancé Windows, il exige `Wow.exe`, ne télécharge aucun client propriétaire, sauvegarde le `realmlist.wtf` existant puis configure uniquement la connexion locale. Le lanceur sait désormais consommer les quatre images RealmBox multiarchitecture précompilées et épinglées par digest : dans ce mode joueur, le Compose ne contient aucun build C++ et télécharge uniquement les images correspondant à la machine. Comme ces images ne sont pas encore publiées, les bundles de développement actuels récupèrent les commits immuables du serveur et de Playerbots puis les construisent localement afin de qualifier exactement les binaires. Dans les deux cas, RealmBox extrait `maps`, `vmaps`, `mmaps` et `dbc` depuis les données locales, initialise MySQL et crée le compte local `REALMBOX / REALMBOX`. Pour l'option dialogue, il récupère aussi le commit épinglé de `mod-ollama-chat`, l'archive Ollama 0.33.2 vérifiée et le modèle autorisé choisi avec CanIRun.

La requête CanIRun contient uniquement le nom du processeur, le nombre de cœurs et la quantité de mémoire. RealmBox teste une liste fermée de petits modèles et réserve au maximum 25 % de la RAM (plafond 8 Go) afin de laisser fonctionner le client, le serveur et les bots. CanIRun donne une estimation, pas un benchmark. Si le service est indisponible ou si aucun modèle n'est confortable, seuls les dialogues LLM sont désactivés ; Playerbots continue de fonctionner.

RealmBox ne télécharge, ne copie dans le dépôt et ne distribue aucune donnée propriétaire. Le dossier `Data` reste à son emplacement d'origine et n'est monté qu'en lecture seule pendant l'extraction.

## Lancements suivants

L'ouverture de RealmBox déclenche automatiquement :

```text
MySQL local → vérification des données serveur → migrations → Ollama local si demandé → authserver/worldserver → client choisi
```

Playerbots est activé uniquement si le joueur l'a demandé. Ollama écoute sur `127.0.0.1:11435`, avec les fonctions cloud désactivées pendant le jeu, une seule requête parallèle et un seul modèle chargé. Les ports de jeu sont liés à `127.0.0.1` et MySQL n'est pas publié sur l'hôte. RealmBox surveille le processus client qu'il a démarré et arrête automatiquement le monde, la base et Ollama quand ce client se ferme. Les journaux, sauvegardes de configuration et composants gérés restent dans le répertoire applicatif `org.realmbox.desktop`.

Dans le jeu, l’addon RealmBox permet de former une équipe équilibrée de quatre bots au niveau du joueur, puis de leur demander de suivre, attaquer, attendre ou se regrouper. Les autres bots continuent de parcourir le monde de façon autonome.

Pendant la partie, la vue **Compagnons** peut modifier la population demandée sans fermer le client. RealmBox recalcule d’abord la limite mémoire, recharge `playerbots.conf` avec la commande Playerbots officielle puis déclenche une mise à jour complète. La connexion ou déconnexion effective des bots peut prendre quelques instants.

## Client selon la plateforme

- **Windows x64** : une copie 3.3.5a avec `Wow.exe` peut être lancée directement ; OpenWoW reste disponible comme option expérimentale.
- **macOS Apple Silicon** : le package Windows fournit les données, mais pas un exécutable natif. RealmBox utilise OpenWoW arm64 pour éviter une machine virtuelle Windows.
- **Linux** : prévu, non implémenté. Les options à qualifier sont OpenWoW natif et le client Windows via Wine.

La matrice et les décisions détaillées sont suivies dans [ROADMAP.md](ROADMAP.md) et [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md).

## Développement

Le développement local observé reste macOS Apple Silicon. Le produit cible aussi Windows x64 ; le chemin Windows compile avec le toolchain MSVC et dispose d'une CI dédiée, mais doit encore être exécuté sur une vraie machine Windows. Le parcours requiert Docker Desktop démarré, Git et curl ; Node, pnpm et Rust ne sont requis que pour construire RealmBox.

```sh
pnpm install
pnpm dev          # application Tauri, commandes réelles
pnpm dev:preview  # aperçu navigateur, aucune simulation d'installation
pnpm verify
python3 -m http.server 1421 --directory site  # aperçu du site Pages
```

Le parcours complet avec données de jeu n'est pas déclaré validé tant qu'une copie utilisateur n'a pas permis de terminer l'installation et d'entrer en jeu. Voir [STATUS.md](STATUS.md) pour la séparation entre tests, build et preuve réelle.

RealmBox est licencié sous AGPL-3.0-only. Les contributions sont décrites dans [CONTRIBUTING.md](CONTRIBUTING.md). La redistribution de binaires et de leurs dépendances doit encore faire l'objet d'une revue juridique avant publication ; aucune donnée WoW n'est acceptée dans les issues, artefacts ou releases.
