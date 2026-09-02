# Intégration mod-ollama-chat

Le commit `a9d14b0...` contient des réglages vérifiés comme `OllamaChat.MaxConcurrentQueries`, `NumPredict`, `NumCtx`, `ThinkMode`, l'historique, les canaux et les chatter. Les presets RealmBox utilisent uniquement ces clés réelles.

Risque principal : le README du commit nomme encore les anciens dépôts `liyunfan1223`, tandis que RealmBox épingle les continuations `mod-playerbots`. Seul un build combiné puis un chargement worldserver peut prouver la compatibilité. Les ordres de gameplay restent séparés de la conversation et bornés par allowlist.

