# Architecture

Le parcours joueur normal passe par cinq commandes Tauri étroites : état initial, conseil matériel, installation, démarrage et arrêt. React ne manipule ni processus, ni Docker, ni secret. `LauncherService` possède la machine à états et dépend d'un `CommandRunner` typé, remplaçable par un enregistreur dans les tests.

```text
React → commandes Tauri → LauncherService → CommandRunner
                              ├─ manifeste d'installation atomique
                              ├─ OpenWoW par plateforme, vérifié par SHA-256
                              │  ou client original Windows fourni par le joueur
                              ├─ Git aux commits exacts
                              ├─ CanIRun (CPU, cœurs, RAM seulement)
                              ├─ Docker Compose local
                              ├─ Ollama local possédé par RealmBox
                              └─ processus client possédé par RealmBox
```

## Premier lancement

L'installation travaille dans `.installing-v3` puis publie atomiquement `runtime-v3`. Le manifeste `installation.json` n'est écrit qu'après la réussite du client, du build serveur, de l'extraction locale, de l'import SQL, de la création du compte et, si demandé, du téléchargement du modèle. En cas d'erreur, RealmBox arrête la composition, supprime ses volumes d'installation incomplets puis retire le staging. Les données du joueur ne sont jamais supprimées.

Le conseil IA ne fait pas confiance aveuglément au classement général CanIRun. RealmBox soumet quatre modèles autorisés à son endpoint de compatibilité, conserve le premier modèle confortable dans l'ordre du plus riche au plus léger, et applique en plus son propre budget de 25 % de RAM plafonné à 8 Go. Aucune chaîne retournée par le service ne devient une commande ou un identifiant de modèle libre.

## Lancements suivants

La présence d'un manifeste valide déclenche `start` automatiquement. Le manifeste persiste le choix du client, son exécutable et l'empreinte éventuelle de l'artefact géré. L'extraction est mise en cache dans un volume portant l'identité du chemin source ; les migrations SQL restent idempotentes. Si les dialogues sont actifs, RealmBox lance son exécutable Ollama sur l'interface loopback avant le monde, avec `OLLAMA_NO_CLOUD=true`. Un superviseur suit le PID du client possédé par le lanceur ; quand ce client disparaît, il déclenche le même arrêt ordonné que le bouton Arrêter et ne touche pas un éventuel processus étranger. Le détail serveur reste dans les logs et diagnostics, pas dans le flux principal.

Les anciens crates domaine/SQLite/fake sont encore présents dans le workspace pour l'historique et leurs tests, mais ne pilotent plus l'application Tauri actuelle.
