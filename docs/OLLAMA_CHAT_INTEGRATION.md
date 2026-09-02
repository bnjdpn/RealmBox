# Intégration mod-ollama-chat

Le commit `a9d14b0...` contient des réglages vérifiés comme `OllamaChat.MaxConcurrentQueries`, `NumPredict`, `NumCtx`, `ThinkMode`, l'historique, les canaux et les chatter. Les presets RealmBox utilisent uniquement ces clés réelles.

RealmBox clone ce module uniquement si le joueur active les dialogues. Ollama 0.33.2 est téléchargé depuis sa release officielle et refusé si son SHA-256 diffère. Le modèle appartient à une allowlist courte ; CanIRun doit le classer confortable et RealmBox applique son propre budget mémoire. Le modèle est téléchargé au premier lancement, puis l'inférence s'effectue sur `127.0.0.1:11435` avec les fonctions cloud coupées.

Le preset privilégie des interventions rares plutôt qu'un brouhaha synthétique : une seule requête simultanée, aucune réponse bot-à-bot ordinaire, un seul bot par événement et des plafonds courts. L'objectif est d'ajouter de la présence aux compagnons, pas de transformer chaque canal en texte généré.

Risque principal : le README du commit nomme encore les anciens dépôts `liyunfan1223`, tandis que RealmBox épingle les continuations `mod-playerbots`. Seul un build combiné puis un chargement worldserver peut prouver la compatibilité. Les ordres de gameplay restent séparés de la conversation et bornés par allowlist.
