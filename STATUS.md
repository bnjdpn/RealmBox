# État factuel

Mis à jour le 2 septembre 2026 sur macOS 26.6.2 arm64.

Décision actuelle : **GO pour fournir au lanceur un dossier `Data` 3.3.5a légitime et terminer le parcours sur ce Mac ; GO code pour préparer un essai Windows x64 ; NO-GO pour affirmer que le jeu complet ou une release distribuable sont déjà validés.**

| Fonction | État | Preuve actuelle |
|---|---|---|
| Lanceur inspiré de l'ère Wrath | implémenté | composition calée sur le launcher 3.3.5a fourni ; illustration fantasy originale, aucune ressource Blizzard ; QA Playwright à 1200 px avec le sélecteur de client |
| Premier lancement | exécuté par composants jusqu'à la frontière des données | OpenWoW réel, quatre images serveur, import SQL, Playerbots et authserver ont été qualifiés ; aucune copie 3.3.5a compatible n'est présente sur ce Mac, donc extracteurs, worldserver complet et connexion en jeu n'ont pas pu être exécutés |
| Client OpenWoW géré | artefacts vérifiés, exécution macOS qualifiée jusqu'aux MPQ | release officielle 0.1.2 ; macOS arm64 SHA-256 `832cb82f…`, signature ad hoc valide et processus réel lancé avec `--game-data` ; arrêt propre et erreurs explicites sur les MPQ absents ; Windows x64 SHA-256 `12e3b92e…`, exécution Windows non faite |
| OpenWoW installé sur ce Mac | réussi | archive officielle 0.1.2 de SHA-256 `832cb82f…` ; bundle recopié dans `/Users/benjamin/Applications/OpenWoW.app` puis signature profonde revérifiée |
| Client original Windows | implémenté, non exécuté sur Windows | vérification de `Wow.exe`, sauvegarde durable du `realmlist.wtf`, configuration loopback, lancement dans le dossier joueur et supervision dédiée ; test de régression des fichiers sur macOS avec effets locaux |
| Serveur AzerothCore Playerbots | images Docker arm64 construites et binaires exécutés | fork `47960183...`, module `2f7d9f77...` ; `authserver` et `worldserver --version` confirment le commit ; quatre images locales produites ; worldserver atteint ensuite la frontière attendue `Failed to find map files for starting areas` |
| Images précompilées pour les joueurs | consommation et bundles CI implémentés, publication absente | CI manuelle x64/ARM64, commits épinglés, contrôle du pull anonyme et artefact de quatre digests ; le même run injecte ces digests dans les bundles macOS arm64/Windows x64 ; le lanceur refuse tags flottants/ensembles partiels et génère un Compose sans `build` ; aucun digest RealmBox n’existe encore, le bundle local courant conserve donc honnêtement son build source |
| Données serveur | prévalidation et extraction locale implémentées, extraction non exécutée | sélection contrôlée avant installation : signatures MPQ et archives `common`/`expansion`/`lichking` plus locale WotLK ; volume Docker géré ; `Data` utilisateur monté en lecture seule ; aucun téléchargement de données extraites |
| MySQL | import réel réussi dans un smoke isolé | image multiarchitecture verrouillée par digest ; aucun port hôte publié ; 22 tables auth, 111 characters, 315 world et 30 Playerbots ; les volumes de smoke ont été supprimés après preuve |
| Compte joueur local | implémenté, non testé contre une base réelle | calcul SRP6 aligné sur la source épinglée et vecteur de régression ; création idempotente `REALMBOX / REALMBOX` |
| Playerbots à la demande | build et base réels validés, comportement en jeu non testé | 50 bots quand activé, zéro et autologin coupé sinon ; configuration complète upstream conservée ; base créée avec 30 tables et 1 908 textes ; observation des bots impossible sans données de monde |
| Conseil CanIRun | API réelle intégrée | inspection CPU/cœurs/RAM dédiée macOS et Windows ; requête limitée à ces informations, allowlist et budget RealmBox testés ; sur le Mac de développement, `qwen3:8b` Q4 est classé confortable, note S, estimation 77 tok/s |
| Dialogues IA locaux | installateur et cycle de vie macOS/Windows implémentés, parcours complet non exécuté | `mod-ollama-chat` au commit `a9d14b0...` ; archives Ollama 0.33.2 macOS/Windows épinglées ; exécution réelle prouvée seulement sur macOS, modèle non téléchargé et dialogue en jeu non testé |
| Second lancement automatique | implémenté et testé avec effets factices | état persisté → base → extraction idempotente → migrations → serveurs → client ; aucune preuve réelle complète |
| Arrêt | implémenté | bouton explicite et supervision du PID OpenWoW ; client → services Docker → Ollama, avec tentative de tous les nettoyages même si l'un échoue ; transition automatique testée avec effets factices, pas de smoke complet réel |
| Vérification locale courante | réussie | `pnpm verify` : typecheck, lint, 6 tests UI, build web, clippy strict et 36 tests Rust ; `actionlint` réussi sur tous les workflows et diff sans erreur d’espace |
| Dépôt GitHub | créé, publication en cours | `bnjdpn/RealmBox`, branche `main` ; présentation français/anglais, contribution, sécurité, templates, Dependabot, CI et CD de prerelease ajoutés localement ; la première CI a exposé une valeur YAML non citée pour les outils Cargo et des chemins d’artefacts incorrects, corrigés avant le second push ; nouvelle CI et passage public pas encore relus |
| Bundle Tauri macOS courant | buildé et lancé visiblement | `RealmBox.app` arm64 resigné ad hoc puis vérifié, exécutable SHA-256 `37b8c6c57611a8c66a50fa5e471114a57b1c32e2cfdb50ce466f8f73c2dce9a1` ; fenêtre du bundle relue via l’accessibilité macOS sur l’état « Données de jeu requises » ; pas de notarisation |
| Windows x64 | code implémenté, exécution absente | vérification croisée du launcher atteint le build de ressources Tauri puis bloque sur `llvm-rc` absent du Mac ; la CI Windows existe mais aucun résultat courant n'a été lu |
| Mac Intel | non pris en charge par l'installateur géré | la release OpenWoW 0.1.2 ne publie pas d'artefact macOS x86-64 |
| Signature de distribution / notarisation | bloqué | certificats absents |

## Ce qui manque pour déclarer le parcours fonctionnel

- fournir à RealmBox une copie utilisateur 3.3.5a valide, absente de ce Mac ;
- laisser les extracteurs locaux terminer depuis cette copie ;
- relancer RealmBox et constater les ports 3724/8085, l'ouverture du client et la connexion avec le compte local ;
- créer un personnage, entrer dans le monde et observer les Playerbots activés/désactivés ;
- avec l'option IA, laisser télécharger le modèle recommandé puis observer une réponse de `mod-ollama-chat` tout en contrôlant la mémoire et la latence ;
- vérifier l'arrêt puis une nouvelle reprise après redémarrage de la machine.
- répéter le parcours sur Windows 11 avec OpenWoW puis avec `Wow.exe`, et vérifier la sauvegarde ainsi que la récupération manuelle du `realmlist.wtf` ;
- exécuter et relire la CI Windows/NSIS sur un commit publié.
- publier les quatre images serveur multiarchitecture après audit des notices, relever leurs digests et faire consommer ces digests par le lanceur joueur afin de supprimer le build C++ du premier lancement.

Les tests automatisés, le build d'une dépendance ou l'ouverture de la fenêtre ne remplacent pas cette preuve réelle.
