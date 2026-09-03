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

## Matrice fonctionnelle OpenWoW 0.1.2

`Observé` signifie vu sur le parcours réel macOS Apple Silicon décrit dans `STATUS.md`. `Non rejoué` et `Non testé` restent des absences de preuve, jamais des échecs supposés.

| Fonction | macOS Apple Silicon | Windows x64 | Linux |
|---|---|---|---|
| Connexion au royaume local | Observé | Non testé | Non testé |
| Création et sélection de personnage | Observé | Non testé | Non testé |
| Déplacement | Observé | Non testé | Non testé |
| Combat et sorts | Non rejoué dans la dernière qualification | Non testé | Non testé |
| Inventaire | Non rejoué | Non testé | Non testé |
| Quêtes | Quête de départ obtenue | Non testé | Non testé |
| Chat FR/EN | Observé avec un bot | Non testé | Non testé |
| Addon RealmBox | Équipe formée ; nouvelle UI à rejouer | Non testé | Non testé |
| Son | Non testé | Non testé | Non testé |
| Réglages graphiques | Non testé | Non testé | Non testé |
| Fenêtré / plein écran | Non testé | Non testé | Non testé |
| Fermeture et relance supervisées | Fermeture observée ; alerte de restauration à durcir | Non testé | Non testé |

Le suivi de qualification restant est l’issue GitHub `Compléter la matrice de compatibilité OpenWoW` du jalon `0.4 — Public beta`.

Sources : [téléchargements ChromieCraft](https://chromiecraft.com/en/downloads/) et [README OpenWoW](https://github.com/rkabachenko/OpenWow-snapshot).
