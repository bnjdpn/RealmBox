# Développer sur macOS

`pnpm dev` lance l'application Tauri et ses commandes réelles. `pnpm dev:preview` ne montre que l'interface dans un navigateur ; les boutons d'installation ne simulent rien dans ce mode.

Pour tester le parcours réel, Docker Desktop doit être installé et démarré. Utiliser uniquement une copie 3.3.5a appartenant au testeur. Les données de jeu, bases utilisateur, secrets et sorties d'extraction ne doivent jamais entrer dans le dépôt.

Les artefacts gérés, volumes Docker et logs se trouvent dans le répertoire applicatif macOS de `org.realmbox.desktop`. Une première installation peut être longue car elle compile le serveur et calcule les cartes de navigation.
