# Installer RealmBox / Install RealmBox

[Français](#français) · [English](#english)

RealmBox ne distribue aucune donnée de World of Warcraft. Le launcher utilise le dossier `Data` d’un client compatible fourni par le joueur, prépare AzerothCore et Playerbots en local, puis lance WoW sur le royaume RealmBox.

RealmBox distributes no World of Warcraft data. The launcher uses the `Data` folder from a compatible player-supplied client, prepares AzerothCore and Playerbots locally, then launches WoW on the RealmBox realm.

## Français

### Configuration requise

- Mac Apple Silicon ou PC Windows x64 ;
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) installé, démarré et doté de mémoire ;
- 24 Gio d’espace libre minimum, plus la taille du modèle de dialogue local facultatif ;
- dossier `Data` complet provenant d’un client WoW compatible ;
- connexion Internet pendant la première installation.

La mémoire attribuée à Docker détermine la population maximale :

| Mémoire Docker | Bots autonomes maximum |
| --- | ---: |
| Moins de 12 Gio | 5 |
| 12 à 19 Gio | 50 |
| 20 à 27 Gio | 100 |
| 28 Gio ou plus | 150 |

### 1. Installer Docker Desktop

Téléchargez [Docker Desktop](https://www.docker.com/products/docker-desktop/), terminez son installation, ouvrez-le et attendez que le moteur Docker soit prêt. Laissez-le actif pendant l’installation et les parties RealmBox.

### 2. Récupérer le client WoW

ChromieCraft propose une page de téléchargement dans chaque langue :

- [client et téléchargements en français](https://chromiecraft.com/fr/telechargements/) ;
- [English client and downloads](https://chromiecraft.com/en/downloads/).

Choisissez le client ou le pack de langue proposé sur la page souhaitée. RealmBox détecte la locale réellement présente dans `Data`. La langue de la page ne remplace pas les fichiers de locale déjà présents dans une autre copie.

Sur Mac Apple Silicon, le package Windows fournit le dossier `Data` ; RealmBox lance ensuite OpenWoW natif géré et ne tente pas d’exécuter `Wow.exe`. Sur Windows x64, le `Wow.exe` du joueur est recommandé lorsqu’il est présent ; OpenWoW géré reste disponible en option.

La cible de compatibilité technique est la build 12340. RealmBox accepte la racine du client ou le dossier `Data` lui-même, vérifie les MPQ requis et laisse les extracteurs locaux confirmer la build exacte. Le dossier source est monté en lecture seule pendant l’extraction.

### 3. Télécharger RealmBox

1. Ouvrez les [releases GitHub](https://github.com/bnjdpn/RealmBox/releases).
2. Prenez le DMG pour Mac Apple Silicon ou l’installateur pour Windows x64.
3. Téléchargez aussi `SHA256SUMS.txt` et comparez l’artefact avant de l’ouvrir :

   ```sh
   # macOS
   shasum -a 256 RealmBox_*.dmg

   # Windows PowerShell ou terminal
   certutil -hashfile RealmBox_*.exe SHA256
   ```

Les binaires distribués actuellement ne sont ni signés ni notariés. Ne contournez pas un avertissement macOS ou Windows si la somme calculée ne correspond pas exactement à la somme publiée.

### 4. Préparer Azeroth

1. Ouvrez RealmBox.
2. Sélectionnez la racine du client WoW ou son dossier `Data`.
3. Choisissez la population de bots souhaitée ; RealmBox la borne automatiquement selon la mémoire visible par Docker.
4. Activez les dialogues locaux uniquement si vous souhaitez télécharger le modèle facultatif proposé.
5. Cliquez sur **Installer**.

Le premier passage vérifie les données, télécharge les composants épinglés, extrait localement `maps`, `vmaps`, `mmaps` et `dbc`, importe les bases, crée le compte local et ne publie le runtime qu’après validation. Cette étape peut être longue ; les lancements suivants réutilisent l’installation locale.

### 5. Jouer

Cliquez sur **Jouer** lorsque RealmBox indique qu’Azeroth est prêt. Le launcher démarre MySQL, l’authserver, le worldserver et le client dans l’ordre requis. La fermeture du client supervisé arrête les services sans supprimer les volumes de personnages.

En cas de problème, utilisez uniquement le diagnostic partageable expurgé puis consultez le [guide de dépannage](TROUBLESHOOTING.md). Ne joignez jamais de MPQ, dump SQL, secret, base utilisateur ou chemin privé non expurgé.

## English

### System requirements

- Apple Silicon Mac or Windows x64 PC;
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) installed, running, and assigned memory;
- at least 24 GiB of free space, plus the optional local dialogue model size;
- a complete `Data` folder from a compatible WoW client;
- internet access during the first installation.

Memory assigned to Docker determines the maximum population:

| Docker memory | Maximum autonomous bots |
| --- | ---: |
| Under 12 GiB | 5 |
| 12–19 GiB | 50 |
| 20–27 GiB | 100 |
| 28 GiB or more | 150 |

### 1. Install Docker Desktop

Download [Docker Desktop](https://www.docker.com/products/docker-desktop/), finish its setup, open it, and wait for the Docker engine to become ready. Keep it running while installing and playing RealmBox.

### 2. Get the WoW client

ChromieCraft provides a download page in each language:

- [client et téléchargements en français](https://chromiecraft.com/fr/telechargements/);
- [English client and downloads](https://chromiecraft.com/en/downloads/).

Choose the client or language pack offered on your preferred page. RealmBox detects the locale actually present in `Data`; changing the page language does not replace locale files already present in another copy.

On Apple Silicon Macs, the Windows package supplies the `Data` folder; RealmBox then launches managed native OpenWoW instead of trying to run `Wow.exe`. On Windows x64, the player's `Wow.exe` is recommended when present; managed OpenWoW remains optional.

The technical compatibility target is build 12340. RealmBox accepts the client root or the `Data` folder itself, checks the required MPQs, and lets the local extraction tools confirm the exact build. The source folder is mounted read-only during extraction.

### 3. Download RealmBox

1. Open [GitHub Releases](https://github.com/bnjdpn/RealmBox/releases).
2. Choose the Apple Silicon Mac DMG or Windows x64 installer.
3. Download `SHA256SUMS.txt` and compare the artifact before opening it:

   ```sh
   # macOS
   shasum -a 256 RealmBox_*.dmg

   # Windows PowerShell or terminal
   certutil -hashfile RealmBox_*.exe SHA256
   ```

Current distributed binaries are not signed or notarized. Do not bypass a macOS or Windows warning unless the calculated checksum exactly matches the published value.

### 4. Prepare Azeroth

1. Open RealmBox.
2. Select the WoW client root or its `Data` folder.
3. Choose the desired bot population; RealmBox automatically caps it against memory visible to Docker.
4. Enable local dialogue only if you want to download the proposed optional model.
5. Select **Install**.

The first pass validates the data, downloads pinned components, locally extracts `maps`, `vmaps`, `mmaps`, and `dbc`, imports the databases, creates the local account, and publishes the runtime only after verification. This can take time; later launches reuse the local installation.

### 5. Play

Select **Play** when RealmBox reports that Azeroth is ready. The launcher starts MySQL, the authserver, worldserver, and client in the required order. Closing the supervised client stops the services without deleting character volumes.

If something fails, use only the redacted shareable diagnostic and see the [troubleshooting guide](TROUBLESHOOTING.md). Never attach an MPQ, SQL dump, secret, user database, or unredacted private path.
