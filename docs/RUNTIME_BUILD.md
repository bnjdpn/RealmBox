# Construction des runtimes

Le lanceur macOS arm64 utilise trois origines immuables : l'archive officielle OpenWoW 0.1.2 vérifiée par SHA-256, le fork serveur Playerbots à un commit exact et le module Playerbots à un commit exact. MySQL est référencé par digest multiarchitecture.

Le serveur est construit localement par Docker depuis les sources épinglées. Les outils AzerothCore extraient `dbc`, `maps`, `vmaps` et `mmaps` depuis le dossier `Data` du joueur monté en lecture seule. Seules les sorties d'extraction entrent dans un volume privé RealmBox ; elles ne sont jamais packagées ou distribuées.

Les commandes `xtask build-*` historiques restent des garde-fous de développement et ne constituent pas le chemin d'installation du produit.
