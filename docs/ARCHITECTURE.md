# Architecture

Le parcours joueur normal passe par des commandes Tauri étroites couvrant l’état initial, le conseil matériel, l’inspection des données, l’installation, la configuration, le démarrage, l’arrêt et le diagnostic. React ne manipule ni processus, ni Docker, ni secret. `LauncherService` possède la machine à états et dépend d'un `CommandRunner` typé, remplaçable par un enregistreur dans les tests.

Les erreurs traversent cette frontière sous forme de `LauncherCommandError` sérialisée : code stable, composant, détail technique optionnel et actions de récupération bornées. React traduit le code et ne déduit plus la cause depuis une phrase française. Le détail brut reste réservé au diagnostic.

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

L'installation travaille dans `.installing-v3` puis publie atomiquement `runtime-v3`. Le manifeste `installation.json` n'est écrit qu'après la réussite du client, du build serveur, de l'extraction locale, de l'import SQL, de la création du compte et, si demandé, du téléchargement du modèle. En cas d'erreur, RealmBox arrête la composition sans jamais demander la suppression des volumes. Une installation est refusée dès qu’un manifeste ou un runtime existe déjà : le parcours initial ne peut pas servir de réinstallation destructive.

Le conseil IA ne fait pas confiance aveuglément au classement général CanIRun. RealmBox soumet quatre modèles autorisés à son endpoint de compatibilité, conserve les modèles confortables dans son propre budget de 25 % de RAM plafonné à 8 Go, puis maximise le débit estimé par gigaoctet officiel parmi les modèles 3B+ ; le 1B reste un repli automatique. Aucune chaîne retournée par le service ne devient une commande ou un identifiant de modèle libre. Après téléchargement, le manifeste Ollama local est haché et comparé au digest immuable du catalogue.

L’activation après installation utilise un staging `runtime-v3/.ai-installing`. Le manifeste d’installation et la configuration du module ne passent à l’état actif qu’après vérification de l’archive Ollama, téléchargement complet et validation du manifeste du modèle. Un monde en cours d’exécution refuse la modification pour éviter d’afficher un état actif que `worldserver` n’aurait pas encore chargé.

## Lancements suivants

La présence d'un manifeste valide affiche l’état prêt sans lancer le monde. `start` ne s’exécute qu’après l’action **Jouer** ; cela laisse au joueur la possibilité de préparer les dialogues avant le démarrage. Le manifeste persiste le choix du client, son exécutable et l'empreinte éventuelle de l'artefact géré. L'extraction est mise en cache dans un volume portant l'identité du chemin source. Avant la première migration de chaque version, RealmBox exporte toutes les bases MySQL, vérifie la présence des quatre bases attendues, calcule le SHA-256 et conserve le dump hors du runtime. Une sauvegarde impossible ou incomplète bloque la migration. Le marqueur de version n’est avancé qu’après la réussite de `db-import`.

Le bootstrap inspecte également les deux volumes persistants quand Docker répond, ainsi que le marqueur de reprise hors Docker. Une purge externe ou une reconstruction interrompue est signalée dans l’état prêt ; au clic sur **Jouer**, `docker-recovery.json` fixe le dump vérifié à restaurer avant la recréation. L’ordre devient alors `MySQL neuf → restauration SQL → validation des quatre bases → sauvegarde de migration éventuelle → données serveur → db-import → client`. Le marqueur n’est retiré qu’une fois le royaume local rendu disponible. Une image locale non retéléchargeable qui a disparu peut être remplacée par les quatre références immuables embarquées dans un bundle de release. La configuration MMaps issue du commit serveur épinglé est elle aussi embarquée, copiée dans le runtime et montée dans l’outil d’extraction ; les anciens fichiers Compose sont migrés de façon idempotente.

Le nom de projet Compose `realmbox-v3` est un identifiant persistant, pas un numéro de release. Il reste stable afin que toutes les versions retrouvent le même volume de base. La construction des arguments Compose rejette explicitement `--volumes` et `-v`. Les détails et limites sont définis dans [UPDATES.md](UPDATES.md).

Si les dialogues sont actifs, RealmBox lance son exécutable Ollama sur l'interface loopback avant le monde, avec `OLLAMA_NO_CLOUD=true`. Un superviseur suit le PID du client possédé par le lanceur ; quand ce client disparaît, il déclenche le même arrêt ordonné que le bouton Arrêter et ne touche pas un éventuel processus étranger. Le détail serveur reste dans les logs et diagnostics, pas dans le flux principal.

Le workspace ne conserve qu’une implémentation produit : l’application Tauri et ses outils. Les anciens crates de démonstration, qui ne pilotaient pas le launcher, ont été retirés afin que les tests du workspace couvrent uniquement du code réellement utilisé.
