# Compatibilité

La cible de données/protocole déclarée par OpenWoW 0.1.2 au commit épinglé est 3.3.5a build 12340. OpenWoW ne distribue aucun asset et demande explicitement au joueur sa propre copie légitime. RealmBox accepte le dossier racine ou son sous-dossier `Data`, puis exige de vraies signatures MPQ dans `common.MPQ`, `expansion.MPQ`, `lichking.MPQ`, `locale-{locale}.MPQ` et `lichking-locale-{locale}.MPQ`. Cela rejette immédiatement les dossiers incomplets ou factices ; la build exacte 12340 reste confirmée ensuite par l’extraction AzerothCore, car les seuls noms d’archives ne constituent pas une preuve de version.

Les locales reconnues sont `frFR`, `enUS`, `enGB`, `deDE`, `esES`, `esMX`, `ruRU`, `koKR`, `zhCN` et `zhTW`. Leur compatibilité complète reste à tester sur des données appartenant aux joueurs.

## Matrice client

| Plateforme | Client 3.3.5a original | OpenWoW | Décision RealmBox |
|---|---|---|---|
| Windows x64 | `Wow.exe` utilisable directement | binaire x64 disponible, expérimental | client original recommandé quand il est présent ; OpenWoW optionnel |
| macOS Apple Silicon | le package Windows fournit `Data`, pas un exécutable natif | binaire arm64 disponible | OpenWoW recommandé pour une exécution native |
| Linux x64/arm64 | client Windows possible via Wine | plateforme annoncée par OpenWoW | futur parcours à tester avant support |
| macOS Intel | anciens chemins historiques ou virtualisation | aucun artefact x86-64 dans la release 0.1.2 | non pris en charge actuellement |

ChromieCraft décrit son téléchargement comme un client 3.3.5a propre avec leur `realmlist`. Sa propre page recommande le client Windows puis, sur Apple Silicon moderne, une machine virtuelle Windows. Ce package peut donc fournir le dossier `Data` à RealmBox et un `Wow.exe` utilisable sur Windows, mais il ne fournit pas à lui seul une exécution native sur Apple Silicon.

OpenWoW annonce Windows, macOS et Linux, lit les données d’une copie existante et ne distribue aucun asset du jeu. Son README qualifie encore le projet d’expérimental ; la compatibilité doit rester prouvée fonction par fonction.

Sources : [téléchargements ChromieCraft](https://chromiecraft.com/en/downloads/) et [README OpenWoW](https://github.com/rkabachenko/OpenWow-snapshot).
