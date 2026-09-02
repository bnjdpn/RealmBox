# Architecture

Le parcours joueur normal passe par quatre commandes Tauri étroites : état initial, installation, démarrage et arrêt. React ne manipule ni processus, ni Docker, ni secret. `LauncherService` possède la machine à états et dépend d'un `CommandRunner` typé, remplaçable par un enregistreur dans les tests.

```text
React → commandes Tauri → LauncherService → CommandRunner
                              ├─ manifeste d'installation atomique
                              ├─ OpenWoW vérifié par SHA-256 et codesign
                              ├─ Git aux commits exacts
                              ├─ Docker Compose local
                              └─ processus client possédé par RealmBox
```

## Premier lancement

L'installation travaille dans `.installing-v1`. Le manifeste `installation.json` n'est écrit qu'après la réussite du client, du build serveur, de l'extraction locale, de l'import SQL et de la création du compte. En cas d'erreur, RealmBox arrête la composition, supprime ses volumes d'installation incomplets puis retire le staging. Les données du joueur ne sont jamais supprimées.

## Lancements suivants

La présence d'un manifeste valide déclenche `start` automatiquement. L'extraction est mise en cache dans un volume portant l'identité du chemin source ; les migrations SQL restent idempotentes. Le détail serveur reste dans les logs et diagnostics, pas dans le flux principal.

Les anciens crates domaine/SQLite/fake sont encore présents dans le workspace pour l'historique et leurs tests, mais ne pilotent plus l'application Tauri actuelle.
