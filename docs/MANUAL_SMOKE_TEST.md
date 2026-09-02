# Smoke tests manuels

Conserver une fiche séparée par plateforme et noter commit, architecture, binaire, hash, date et résultat observé.

1. Valider les données utilisateur sans les modifier ni les téléverser.
2. Démarrer la base et prouver un ping authentifié local.
3. Démarrer authserver/worldserver et observer leurs marqueurs de disponibilité.
4. Vérifier le chargement Playerbots et mod-ollama-chat.
5. Vérifier Ollama sur localhost, le modèle exact et une réponse bornée.
6. Lancer OpenWoW, se connecter, choisir un personnage et entrer dans le monde.
7. Former un groupe équilibré, suivre, combattre, accomplir une quête et dialoguer.
8. Fermer le client et vérifier que seuls les processus appartenant à RealmBox sont arrêtés.
9. Relancer et vérifier la reprise du personnage et l'absence de processus orphelin.

Rien dans cette liste n'a encore été exécuté avec le runtime réel.

