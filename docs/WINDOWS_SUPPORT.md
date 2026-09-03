# Support Windows

Windows x64 possède maintenant un chemin d'installation dédié : archive officielle OpenWoW 0.1.2 épinglée par SHA-256, extraction PowerShell, jonction NTFS vers les données du joueur, inspection CPU/mémoire, archive Ollama Windows vérifiée et environnement de processus local. Les téléchargements joueur utilisent le client HTTP Rust, avec reprise et contrôle SHA-256, sans dépendance à `curl`. Les processus client et Ollama sont associés à des Job Objects `KILL_ON_JOB_CLOSE` et seuls les handles créés par RealmBox peuvent être arrêtés. Ce code Windows compile dans un harnais MSVC isolé ; le build Tauri croisé depuis macOS reste arrêté avant le crate par l’absence de `llvm-rc`, disponible sur le runner Windows.

Le premier lancement propose deux clients :

- **OpenWoW géré**, recommandé et téléchargé par RealmBox ;
- **client original fourni par le joueur**, jamais téléchargé par RealmBox. Ce mode vérifie `Wow.exe`, sauvegarde le `realmlist.wtf` d'origine dans les données applicatives RealmBox, pointe le client vers `127.0.0.1` et l'exécute depuis son dossier.

Ce code et ses tests automatisés ne valent pas validation Windows réelle. La restauration automatique runtime + SQL et le retour à l’état initial après import défaillant sont couverts avec un runner factice ; ils doivent encore être rejoués sur une base Windows réelle. Credential Manager n’est pas intégré.

## Fiche de qualification Windows 11 obligatoire

La fiche complétée doit conserver le commit, la version et le SHA-256 de l’installateur, l’édition/build Windows, la version Docker Desktop, le matériel et chaque résultat observé. Une case vide ou un résultat indirect maintient Windows au statut expérimental.

- [ ] compte utilisateur standard, sans élévation permanente ;
- [ ] Defender et SmartScreen actifs ;
- [ ] chemin avec espaces, accents et nom d’utilisateur non ASCII ;
- [ ] affichage à 125 %, 150 % puis 200 % ;
- [ ] installation OpenWoW gérée, connexion et entrée en jeu ;
- [ ] installation avec le `Wow.exe` fourni par le joueur ;
- [ ] compagnons activés, désactivés et plafonnés selon la mémoire ;
- [ ] dialogues activés, désactivés et fonctionnement hors ligne après installation ;
- [ ] fermeture normale, kill brutal du client et redémarrage Windows ;
- [ ] vérification qu’aucun processus RealmBox n’est orphelin et qu’aucun processus non possédé n’est arrêté ;
- [ ] sauvegarde, migration volontairement défaillante, restauration SQL + runtime et vérification en jeu ;
- [ ] désinstallation du launcher en conservant le royaume et ses volumes ;
- [ ] réinstallation puis redécouverte du royaume existant ;
- [ ] restauration du `realmlist.wtf` d’origine ;
- [ ] build et installation NSIS signés, validation Authenticode et SmartScreen ;
- [ ] Narrator et clavier sur le parcours principal et la modale.
