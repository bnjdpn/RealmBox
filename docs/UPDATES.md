# Mises à jour

Les données des joueurs ont priorité sur la disponibilité d’une mise à jour. RealmBox doit échouer fermé : s’il ne peut pas prouver qu’une sauvegarde complète existe, il ne lance aucune migration.

## Invariants obligatoires

- Le projet Docker reste `realmbox-v3`. Ce nom fait partie du format persistant et ne doit jamais être versionné avec l’application : le changer ferait apparaître un volume vide à la place du royaume existant.
- Aucune commande contrôlée par RealmBox n’accepte `docker compose down --volumes` ou `-v`. L’arrêt retire les conteneurs, jamais les volumes.
- Le parcours d’installation refuse de démarrer si `installation.json` ou `runtime-v3` existe. Il n’existe pas de « réinstallation » destructive déguisée en mise à jour.
- Un manifeste d’un schéma inconnu produit une erreur conservatrice. Il n’est jamais interprété comme une absence d’installation.
- Avant la première migration exécutée par chaque version de RealmBox, MySQL produit un dump cohérent de toutes les bases avec `--single-transaction`, routines, événements, déclencheurs et données binaires.
- Toute version distribuée doit incrémenter la version du package Cargo desktop : ce numéro déclenche la sauvegarde obligatoire, même si le schéma SQL annoncé reste identique.
- Le dump doit nommer `acore_auth`, `acore_characters`, `acore_playerbots` et `acore_world`. Un dump vide ou incomplet bloque la migration.
- Chaque dump est conservé dans `player-data-backups`, hors de `runtime-v3`, avec son SHA-256. Une sauvegarde existante n’est jamais écrasée ; elle est relue et revérifiée avant réutilisation.
- La vue joueur **Protection** réutilise exactement ce contrat complet pour les points créés à la demande. Elle produit toujours un nouveau couple `.sql`/`.sha256`. Si la base est arrêtée, RealmBox démarre et arrête uniquement le service `database` ; si elle tourne déjà avec le monde, la transaction cohérente s’effectue sans interrompre la partie.
- Le marqueur de version migrée n’est enregistré qu’après la réussite de `db-import`. Un échec laisse donc la sauvegarde et l’ancien marqueur intacts.
- Si Docker Desktop a été purgé hors de RealmBox, la disparition du volume `realmbox-v3_realmbox-database` déclenche une récupération dédiée. RealmBox sélectionne la sauvegarde SQL complète et vérifiée la plus récente, écrit un marqueur de reprise hors de Docker, recrée les ressources puis restaure et revalide les quatre bases avant toute migration ou ouverture du client.
- Une récupération Docker interrompue reprend depuis son marqueur. Si aucun dump vérifié n’est disponible, le lancement échoue fermé : un manifeste existant n’est jamais transformé en royaume vide.
- La reconstruction des données serveur utilise la configuration MMaps embarquée depuis le commit AzerothCore épinglé. RealmBox ajoute automatiquement son montage aux anciens fichiers Compose sans changer les volumes ni le nom du projet.
- Tant que `extraction-version` n’est pas publié, une reprise régénère l’intermédiaire `Buildings` et les répertoires VMaps/MMaps dérivés qui peuvent être partiels. Cette opération reste confinée au volume de données serveur remplaçable et ne touche jamais au volume `realmbox-database`.

## Ordre imposé au démarrage après mise à jour

```text
MySQL → sauvegarde SQL vérifiée → données serveur → migration SQL → marqueur de version → auth/world → client
```

Si la sauvegarde ou la migration échoue, le client et les serveurs de jeu ne sont pas démarrés. Les fichiers `.sql` et `.sha256` sont des artefacts locaux privés : ils contiennent les comptes et personnages et ne doivent jamais être joints à une issue, un build ou une release.

## Remplacement du runtime serveur

L’ajout des dialogues à une installation antérieure prépare un nouveau dossier serveur dans un staging séparé. RealmBox télécharge les images serveur immuables, en extrait la configuration du module épinglé et vérifie la configuration avant toute publication.

Après la sauvegarde SQL complète et vérifiée, la composition courante est arrêtée sans supprimer de volume. Le dossier serveur actif est déplacé dans `runtime-rollbacks`, hors de `runtime-v3`, puis le staging est publié par renommage sur le même système de fichiers. Le projet Compose reste `realmbox-v3`, donc les volumes `realmbox-v3_realmbox-database` et `realmbox-v3_realmbox-server-data` restent attachés au royaume. Un rollback existant n’est jamais écrasé.

Le manifeste ne marque pas la migration comme terminée pendant ce remplacement. Au prochain démarrage, la sauvegarde est relue ou recréée pour la transition exacte, `db-import` doit réussir, puis seulement le marqueur de version est avancé.

## Portée actuelle

Ces garde-fous protègent une installation existante lors d’un changement de version du lanceur, avant les migrations de démarrage et pendant la mise à niveau serveur requise par les dialogues. Ils permettent aussi au joueur de créer un point vérifié supplémentaire, qui devient éligible à la récupération Docker s’il est le plus récent. Le rollback est conservé localement ; sa restauration automatique depuis l’interface n’est pas encore revendiquée.
