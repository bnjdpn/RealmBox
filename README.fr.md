# RealmBox

**World of Warcraft. Azeroth sur votre ordinateur. Des bots inclus.**

[Site web](https://bnjdpn.github.io/RealmBox/fr/) · [Releases](https://github.com/bnjdpn/RealmBox/releases) · [Installation](docs/INSTALLATION.md) · [English](README.md)

RealmBox est un launcher desktop libre pour jouer à World of Warcraft entièrement en local. Donnez-lui le dossier `Data` d’un client WoW compatible : il prépare un royaume AzerothCore privé, peuple Azeroth avec Playerbots, lance le jeu et supervise tout le runtime local.

![RealmBox prêt à lancer World of Warcraft en local](site/public/assets/launcher-ready-fr.webp)

## Ce que fournit le projet

- une seule application orientée joueur pour installer, lancer, arrêter, configurer et diagnostiquer ;
- un serveur d’authentification et un monde AzerothCore locaux avec MySQL ;
- des Playerbots autonomes dont une majorité de même faction converge progressivement près des joueurs actifs, et une équipe de compagnons contrôlable en jeu ;
- OpenWoW géré sur macOS Apple Silicon, ainsi que `Wow.exe` ou OpenWoW sur Windows x64 ;
- des dialogues locaux facultatifs, bornés en débit, entre joueurs et bots ou entre bots via un runtime Ollama géré par RealmBox ;
- une installation atomique, des images serveur immuables, des personnages persistants et des sauvegardes vérifiées avant migration.

RealmBox ne contient aucun client World of Warcraft, MPQ, carte extraite, identifiant, base de personnages ni autre donnée propriétaire du jeu. Ces fichiers proviennent de la copie compatible du joueur et restent en local.

## Architecture

```text
React 19 + TypeScript
        │ commandes Tauri étroites
        ▼
LauncherService Rust ── interfaces typées de plateforme et de runtime
        ├── OpenWoW ou Wow.exe appartenant au joueur
        ├── extraction locale depuis le dossier Data monté en lecture seule
        ├── projet Docker Compose : realmbox-v3
        │     ├── MySQL
        │     ├── AzerothCore authserver
        │     ├── AzerothCore worldserver + mod-playerbots
        │     └── outils d’import SQL et d’extraction
        ├── addon RealmBox Compagnons
        └── processus Ollama local facultatif
```

L’interface React ne pilote jamais directement les processus, Docker ou les secrets. Tauri expose une surface de commandes réduite adossée à `LauncherService` ; les effets de plateforme et de runtime passent par des interfaces typées remplaçables par des fakes dans les tests macOS.

Le cycle normal est le suivant :

```text
choisir Data → valider les MPQ → préparer le runtime → extraire localement
→ importer les bases → publier l’installation atomiquement → Jouer
→ démarrer les services → lancer WoW → fermeture du client supervisé
→ arrêter les services sans supprimer les volumes
```

Les images serveur et les révisions des dépendances tierces sont épinglées dans [`third-party.lock.toml`](third-party.lock.toml). Le manifeste d’installation n’est publié qu’après vérification de chaque composant requis.

## Client de jeu

RealmBox accepte la racine du client ou son dossier `Data`. La cible de compatibilité technique est WoW 3.3.5a build 12340 : le launcher vérifie les MPQ attendus, détecte la locale et laisse les extracteurs AzerothCore confirmer la build exacte. Cet identifiant décrit une contrainte de compatibilité du client, pas le produit.

ChromieCraft propose des pages de téléchargement dans les deux langues :

- [Client et téléchargements en français](https://chromiecraft.com/fr/telechargements/)
- [English client and downloads](https://chromiecraft.com/en/downloads/)

Choisissez le client ou le pack de langue proposé sur la page souhaitée. RealmBox utilise ensuite la locale réellement présente dans `Data`. Sur Mac Apple Silicon, le package Windows téléchargé fournit les données du jeu et RealmBox lance OpenWoW natif géré. Sur Windows x64, le `Wow.exe` du joueur est le chemin privilégié lorsqu’il est présent ; OpenWoW géré reste disponible en option.

## Configuration requise

- Mac Apple Silicon ou PC Windows x64 ;
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) installé et démarré ;
- au moins 24 Gio d’espace disque libre, plus la taille du modèle de dialogue local facultatif ;
- un dossier `Data` WoW complet et compatible ;
- une connexion Internet pour la première installation.

La limite de bots dépend de la mémoire attribuée à Docker, pas de la mémoire totale de l’ordinateur :

| Mémoire Docker | Bots autonomes maximum |
| --- | ---: |
| Moins de 12 Gio | 5 |
| 12 à 19 Gio | 50 |
| 20 à 27 Gio | 100 |
| 28 Gio ou plus | 150 |

## Installation

1. Installez et démarrez [Docker Desktop](https://www.docker.com/products/docker-desktop/).
2. Téléchargez les données WoW compatibles depuis la page ChromieCraft [française](https://chromiecraft.com/fr/telechargements/) ou [anglaise](https://chromiecraft.com/en/downloads/).
3. Téléchargez RealmBox depuis les [releases GitHub](https://github.com/bnjdpn/RealmBox/releases) et comparez l’artefact avec `SHA256SUMS.txt`.
4. Ouvrez RealmBox et sélectionnez le dossier qui contient `Data`.
5. Choisissez la population de bots et, si vous le souhaitez, les dialogues locaux, puis cliquez sur **Installer**.
6. Cliquez sur **Jouer** lorsque RealmBox indique qu’Azeroth est prêt.

Les binaires distribués actuellement ne sont ni signés ni notariés. Ne contournez pas un avertissement du système si le SHA-256 de l’artefact téléchargé ne correspond pas exactement à la somme publiée. Le [guide d’installation complet](docs/INSTALLATION.md) détaille chaque plateforme.

## Persistance et sécurité des mises à jour

Le projet Docker Compose se nomme définitivement `realmbox-v3` ; ce n’est pas un numéro de version de l’application. Sa stabilité permet de retrouver le volume de la base joueurs après chaque release.

Avant la première migration exécutée par chaque version desktop, RealmBox :

1. exporte toutes les bases MySQL attendues dans un dump cohérent ;
2. vérifie le contenu du dump et son SHA-256 ;
3. stocke la sauvegarde hors du runtime remplaçable sans écraser une sauvegarde existante ;
4. applique la migration ;
5. n’avance le marqueur de version migrée qu’après réussite.

Un schéma d’installation inconnu, une sauvegarde incomplète ou une migration en échec bloque la mise à jour. Le launcher ne transforme jamais un royaume existant en installation neuve et n’appelle jamais `docker compose down --volumes` ou `-v`.

## Organisation du dépôt

| Chemin | Rôle |
| --- | --- |
| `apps/desktop/` | Interface React et application desktop Tauri |
| `apps/desktop/src-tauri/` | Machine à états Rust et intégrations plateforme/runtime |
| `addons/RealmBoxCompanions/` | Contrôle des compagnons en jeu |
| `runtime/` | Templates Compose, manifestes et helpers de plateforme |
| `patches/` | Correctifs relus appliqués aux composants upstream épinglés |
| `site/` | Sources Astro du site GitHub Pages bilingue |
| `scripts/` | Outils de release, captures, manifestes et validation |
| `tools/xtask/` | Vérification des invariants du dépôt et des releases |
| `docs/` | Architecture, installation, compatibilité, sécurité et exploitation |

## Développement

Le workspace utilise pnpm, Node.js, Rust et Tauri. Docker Desktop est nécessaire pour le parcours local réel.

```sh
pnpm install
pnpm dev:preview   # interface React avec runtime simulé
pnpm dev           # application desktop Tauri
pnpm site:dev      # site GitHub Pages
pnpm verify        # UI, scripts, builds, site, Rust et invariants de release
```

Commandes ciblées utiles :

```sh
pnpm typecheck
pnpm test
pnpm site:build
cargo test --workspace
cargo xtask release check
```

## Documentation technique

- [Architecture](docs/ARCHITECTURE.md)
- [Installation](docs/INSTALLATION.md)
- [Compatibilité](docs/COMPATIBILITY.md)
- [Mises à jour et sauvegardes](docs/UPDATES.md)
- [Sécurité](docs/SECURITY.md)
- [Compilation](docs/BUILDING.md)
- [Dépannage](docs/TROUBLESHOOTING.md)
- [Distribution et licences](docs/LEGAL_AND_DISTRIBUTION.md)

## Licence et indépendance

RealmBox est distribué sous [AGPL-3.0-only](LICENSE). C’est un projet indépendant, sans affiliation ni approbation de Blizzard Entertainment ou de ChromieCraft. World of Warcraft, Azeroth et les noms associés appartiennent à leurs ayants droit respectifs.
