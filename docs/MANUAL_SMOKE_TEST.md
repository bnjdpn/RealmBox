# Smoke tests manuels

Conserver une fiche séparée par plateforme et noter commit, architecture, binaire, hash, date et résultat observé.

1. Valider les données utilisateur sans les modifier ni les téléverser.
2. Démarrer la base et prouver un ping authentifié local.
3. Démarrer authserver/worldserver et observer leurs marqueurs de disponibilité.
4. Vérifier le chargement de Playerbots, d'abord activé puis désactivé.
5. Lancer OpenWoW, se connecter avec le compte local, choisir un personnage et entrer dans le monde.
6. Observer des bots, combattre et changer de zone.
7. Fermer le client et vérifier que seuls les processus appartenant à RealmBox sont arrêtés.
8. Relancer et vérifier la reprise du personnage et l'absence de processus orphelin.

Le parcours macOS Apple Silicon a été exécuté avec le runtime réel jusqu’à la connexion, au personnage, aux quêtes, aux bots et à l’arrêt ; les preuves datées sont consignées dans [STATUS.md](../STATUS.md). Cette fiche reste entièrement à exécuter sur Windows 11 et doit être rejouée après tout changement de restauration ou de supervision.
