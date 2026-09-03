# Feuille de route

Cette liste décrit le produit attendu. Elle ne vaut pas preuve de fonctionnement ; les preuves réelles restent dans [STATUS.md](STATUS.md).

## P0 — Parcours joueur fiable

- [x] Installer et démarrer un royaume 3.3.5a local sur macOS Apple Silicon.
- [x] Entrer en jeu avec OpenWoW, créer un personnage et prendre la quête de départ.
- [x] Adapter automatiquement la population Playerbots à la mémoire allouée à Docker.
- [x] Former depuis l’addon une équipe visible de quatre compagnons au niveau du joueur.
- [x] Réduire l’addon dans une icône de minimap, conserver son état, afficher la composition du groupe et proposer ses contrôles en français et en anglais.
- [x] Gérer la population Playerbots à chaud depuis RealmBox, sans redémarrer le client.
- [x] Séparer les populations proposées `5`, `25`, `50`, `100` et `150` de la présence avec les choix `Dispersés`, `Naturelle` recommandé et `Toujours proches`, applicables à chaud ou enregistrables pour la prochaine partie.
- [x] Préserver `Toujours proches` pour une installation antérieure à 0.4.0 sans préférence et choisir `Naturelle` sur une installation neuve.
- [x] Rendre chaque bot déplacé au scheduler Playerbots avec `ScheduleTeleport`, sans déplacer immédiatement un bot groupé, en danger ou encore visible.
- [x] Protéger les personnages contre les mises à jour : réinstallation refusée, suppression de volume interdite et sauvegarde SQL vérifiée avant migration.
- [x] Permettre au joueur de créer à la demande une sauvegarde complète, vérifiée, non écrasante et utilisable par la récupération Docker, sans fermer un monde déjà ouvert.
- [x] Préparer et publier atomiquement le serveur compatible dialogues en conservant l’ancien runtime hors du dossier actif.
- [x] Ajouter la restauration automatique depuis le rollback conservé, avec sauvegarde de sécurité et retour automatique si l’import échoue.
- [x] Afficher une erreur courte, sa cause concrète et l’action de récupération ; réserver les détails techniques au diagnostic.
- [ ] Afficher une progression d’installation par composant avec débit, volume restant et durée estimée quand ces valeurs sont mesurables.
- [x] Publier les images serveur multiarchitecture épinglées pour supprimer la compilation C++ du parcours joueur.
- [ ] Tester le parcours complet sur Windows 11 avec `Wow.exe` et OpenWoW.

## P1 — Interface du launcher

- [x] Refaire l’interface avec une identité fantasy originale : cadre, état courant, progression et action principale.
- [x] Supprimer les slogans et les phrases décoratives. Chaque texte doit indiquer un état, une action, une capacité ou une erreur.
- [x] Utiliser des ressources originales ou redistribuables ; ne pas intégrer d’illustration, logo, police ou texture Blizzard au dépôt.
- [x] Ajouter une vue Compagnons : population demandée, population réellement appliquée, présence, résumé des comportements d’équipe et actions sûres.
- [ ] Afficher dans cette vue la mémoire réellement utilisée par le worldserver lorsqu’elle est mesurable.
- [x] Ajouter une vue Diagnostic séparée : journaux filtrés, composant fautif, copie du diagnostic et chemin des logs.
- [x] Publier un site GitHub Pages FR/EN avec tutoriel, téléchargements, limites de plateforme et aide de premier niveau.

## P2 — Profils matériels

- [ ] Profils `Petit`, `Équilibré`, `Dense` et `Personnalisé` couvrant bots, mémoire Docker, extraction et IA locale.
- [x] Refuser une configuration dangereuse et montrer séparément la population souhaitée et la valeur sûre réellement appliquée.
- [ ] Mesurer le démarrage réel et proposer un profil inférieur si le serveur dépasse son budget mémoire.
- [x] Conserver séparément le choix du joueur et la valeur effectivement appliquée.

## P3 — Dialogues locaux

- [x] Intégrer Ollama et `mod-ollama-chat` derrière une liste de modèles autorisés et une écoute locale.
- [x] Faire décider RealmBox depuis CanIRun, puis télécharger le modèle sur demande avec sa taille annoncée avant confirmation et son manifeste vérifié par digest.
- [x] Afficher l’espace disque disponible et refuser le téléchargement si la marge est insuffisante.
- [ ] Requalifier dans OpenWoW les modes `Direct`, `Immersif` et `Vivant`, puis mesurer RAM, latence, débit et cadence perçue.
- [x] Permettre d’activer et désactiver les dialogues sans réinstaller le royaume, monde fermé pour recharger le module.
- [x] Permettre de changer à chaud le mode de discussion lorsque le modèle local est actif, avec trois presets bornés.
- [x] Placer les demandes humaines éligibles avant le travail ambiant et leur réserver un emplacement sans rendre la file illimitée.
- [x] Isoler les budgets de portée Party/Raid par groupe tout en conservant le plafond global.
- [x] Désactiver l’historique, la mémoire, les relations, le RAG, le sentiment et les emotes générées dans les trois profils RealmBox.
- [x] Sélectionner les prompts ambiants français pour une copie client `frFR` et anglais pour les autres locales prises en charge.
- [ ] Vérifier en parcours réel la priorité sous charge, deux groupes simultanés, le rebond bot-à-bot et l’absence de flood.
- [x] Configurer un mode sans réseau pendant le jeu et rendre ce statut visible.

## Choix du client

- **Windows x64** : le `Wow.exe` 3.3.5a d’une copie compatible peut être utilisé directement. OpenWoW doit rester une option, pas une obligation.
- **macOS Apple Silicon moderne** : les données du client Windows sont utilisables, mais son exécutable ne tourne pas nativement. OpenWoW arm64 est le chemin natif retenu ; la solution indiquée par ChromieCraft est sinon une machine virtuelle Windows.
- **Linux** : le client Windows peut fonctionner via Wine ; OpenWoW est aussi conçu pour Linux. RealmBox Linux reste à implémenter et tester.
- **Mac Intel** : hors parcours géré tant qu’un binaire OpenWoW x86-64 distribué et vérifié n’est pas disponible.

Le téléchargement ChromieCraft fournit une copie complète configurée pour leur serveur. RealmBox peut reconnaître son dossier `Data` et, sur Windows, son `Wow.exe`. Il ne remplace pas OpenWoW pour une exécution native sur Apple Silicon. RealmBox ne redistribue pas cette copie propriétaire.
