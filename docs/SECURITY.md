# Sécurité

Tous les ports publiés écoutent uniquement sur loopback. RealmBox ne demande pas root/admin, ne crée pas de règle firewall et ne désactive aucune protection système. L'archive OpenWoW est vérifiée par SHA-256 et signature avant exécution ; les sources serveur sont vérifiées au commit exact avant build.

Le mot de passe MySQL est aléatoire, écrit avec le mode `0600` et transmis aux conteneurs par environnement Compose, jamais comme argument de processus. Le compte joueur `REALMBOX / REALMBOX` est un identifiant local non privilégié explicitement affiché, pas un secret. Le client lancé est suivi par son PID en mémoire ; la persistance robuste de l'identité du processus après crash du lanceur reste à faire. Aucun arrêt par nom n'est utilisé.

Signaler les vulnérabilités sans publier de secret ou de donnée propriétaire.
