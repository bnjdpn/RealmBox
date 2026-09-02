# Compatibilité

La cible de données/protocole déclarée par OpenWoW 0.1.2 au commit épinglé est 3.3.5a build 12340. OpenWoW ne distribue aucun asset et demande explicitement au joueur sa propre copie légitime. RealmBox accepte le dossier racine ou son sous-dossier `Data`, puis exige de vraies signatures MPQ dans `common.MPQ`, `expansion.MPQ`, `lichking.MPQ`, `locale-{locale}.MPQ` et `lichking-locale-{locale}.MPQ`. Cela rejette immédiatement les dossiers incomplets ou factices ; la build exacte 12340 reste confirmée ensuite par l’extraction AzerothCore, car les seuls noms d’archives ne constituent pas une preuve de version.

Les locales reconnues sont `frFR`, `enUS`, `enGB`, `deDE`, `esES`, `esMX`, `ruRU`, `koKR`, `zhCN` et `zhTW`. Leur compatibilité complète reste à tester sur des données appartenant aux joueurs.

Le mode client historique est une abstraction Windows avancée désactivée. Wine, Whisky, CrossOver et les machines virtuelles sont hors périmètre.
