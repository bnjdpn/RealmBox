# Construction des runtimes

Le lanceur macOS arm64 utilise des origines immuables : les archives officielles OpenWoW 0.1.2 et Ollama 0.33.2 vérifiées par SHA-256, le fork serveur Playerbots, le module Playerbots et `mod-ollama-chat` à des commits exacts. MySQL est référencé par digest multiarchitecture. Ollama et le module de dialogue ne sont récupérés que lorsque l'option IA est activée.

Le serveur est construit localement par Docker depuis les sources épinglées. Les outils AzerothCore extraient `dbc`, `maps`, `vmaps` et `mmaps` depuis le dossier `Data` du joueur monté en lecture seule. Seules les sorties d'extraction entrent dans un volume privé RealmBox ; elles ne sont jamais packagées ou distribuées.

Les commandes `xtask build-*` historiques restent des garde-fous de développement et ne constituent pas le chemin d'installation du produit.
