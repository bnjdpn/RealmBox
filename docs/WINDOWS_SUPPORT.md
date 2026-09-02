# Support Windows

Windows x64 possède maintenant un chemin d'installation dédié : archive officielle OpenWoW 0.1.2 épinglée par SHA-256, extraction PowerShell, jonction NTFS vers les données du joueur, arrêt et supervision via les outils Windows, inspection CPU/mémoire, archive Ollama Windows vérifiée et environnement de processus local. La compilation Rust du launcher atteint le build Tauri depuis macOS ; elle s'arrête ensuite normalement faute de `llvm-rc`, disponible sur le runner Windows.

Le premier lancement propose deux clients :

- **OpenWoW géré**, recommandé et téléchargé par RealmBox ;
- **client original fourni par le joueur**, jamais téléchargé par RealmBox. Ce mode vérifie `Wow.exe`, sauvegarde le `realmlist.wtf` d'origine dans les données applicatives RealmBox, pointe le client vers `127.0.0.1` et l'exécute depuis son dossier.

Ce code et ses frontières factices ne valent pas validation Windows réelle. Il reste à exécuter sur Windows 11 : installation Docker complète, extraction des cartes, démarrage MySQL/auth/world, connexion des deux clients, Playerbots, Ollama, fermeture supervisée, récupération manuelle de la configuration client sauvegardée, build NSIS, SmartScreen et désinstallation. Les Job Objects, la restauration automatique et Credential Manager ne sont pas encore intégrés.
