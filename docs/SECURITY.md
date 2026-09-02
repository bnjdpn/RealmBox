# Sécurité

Tous les services réels devront écouter uniquement sur loopback. RealmBox ne demande pas root/admin, ne crée pas de règle firewall, ne désactive aucune protection système et ne lance aucun artefact avant vérification.

Les secrets doivent rester dans Keychain ou Credential Manager. Les mots de passe ne passent ni par arguments, ni presse-papiers, ni logs. Les processus sont suivis par PID, identité binaire et jeton de propriété ; aucun arrêt par nom n'est autorisé.

Signaler les vulnérabilités sans publier de secret ou de donnée propriétaire.

