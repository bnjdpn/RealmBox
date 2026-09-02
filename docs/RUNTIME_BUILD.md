# Construction des runtimes

Le lanceur macOS arm64 utilise des origines immuables : les archives officielles OpenWoW 0.1.2 et Ollama 0.33.2 vérifiées par SHA-256, le fork serveur Playerbots, le module Playerbots et `mod-ollama-chat` à des commits exacts. MySQL est référencé par digest multiarchitecture. Ollama et le module de dialogue ne sont récupérés que lorsque l'option IA est activée.

Le build source local actuel est un chemin de qualification, pas le parcours joueur cible. Les releases RealmBox doivent télécharger par digest des images multiarchitecture produites une seule fois par la CI à partir des mêmes commits épinglés. Le workflow est présent, mais aucune image n'est encore déclarée publiée : le lanceur conserve donc provisoirement le build local au lieu d'inventer une référence de registre.

Les outils AzerothCore extraient `dbc`, `maps`, `vmaps` et `mmaps` depuis le dossier `Data` du joueur monté en lecture seule. Ces outils peuvent être précompilés ; les données et les sorties d'extraction restent locales et n'entrent jamais dans une image distribuée.

Les commandes `xtask build-*` historiques restent des garde-fous de développement et ne constituent pas le chemin d'installation du produit.
