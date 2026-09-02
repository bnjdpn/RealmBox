# Architecture

La couche domaine contient l'état et les intentions sans dépendance système. `realmbox-orchestrator` enchaîne et persiste chaque transition. Les effets sont injectés via `ClientBackend` et les interfaces de `realmbox-runtime`. `realmbox-storage` utilise SQLite avec migrations transactionnelles. Tauri expose des commandes étroites à React ; React ne connaît ni processus, ni base serveur, ni secret.

```text
React → commandes Tauri → Orchestrator → Domain
                              ├─ StateStore (SQLite)
                              ├─ ClientBackend
                              └─ interfaces runtime/plateforme
```

Le runtime fake implémente les mêmes frontières, sans drapeaux `if fake` dispersés. Le backend OpenWoW réel valide déjà la structure minimale et refuse de lancer tant qu'aucun binaire réel n'est installé.

