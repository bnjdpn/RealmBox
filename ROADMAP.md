# Feuille de route

Cette liste décrit le produit attendu. Elle ne vaut pas preuve de fonctionnement ; les preuves réelles restent dans [STATUS.md](STATUS.md).

## P0 — Parcours joueur fiable

- [x] Installer et démarrer un royaume 3.3.5a local sur macOS Apple Silicon.
- [x] Entrer en jeu avec OpenWoW, créer un personnage et prendre la quête de départ.
- [x] Adapter automatiquement la population Playerbots à la mémoire allouée à Docker.
- [x] Former depuis l’addon une équipe visible de quatre compagnons au niveau du joueur.
- [x] Réduire l’addon dans une icône de minimap, conserver son état, afficher la composition du groupe et proposer ses contrôles en français et en anglais.
- [x] Gérer la population Playerbots à chaud depuis RealmBox, sans redémarrer le client.
- [x] Protéger les personnages contre les mises à jour : réinstallation refusée, suppression de volume interdite et sauvegarde SQL vérifiée avant migration.
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
- [ ] Ajouter une vue Compagnons : population demandée, population réellement active, équipe, mémoire utilisée et actions sûres.
- [x] Ajouter une vue Diagnostic séparée : journaux filtrés, composant fautif, copie du diagnostic et chemin des logs.
- [x] Publier un site GitHub Pages FR/EN avec tutoriel, téléchargements, limites de plateforme et aide de premier niveau.

## P2 — Profils matériels

- [ ] Profils `Petit`, `Équilibré`, `Dense` et `Personnalisé` couvrant bots, mémoire Docker, extraction et IA locale.
- [x] Refuser une configuration dangereuse et montrer séparément la population souhaitée et la valeur sûre réellement appliquée.
- [ ] Mesurer le démarrage réel et proposer un profil inférieur si le serveur dépasse son budget mémoire.
- [ ] Conserver séparément le choix du joueur et la valeur effectivement appliquée.

## P3 — Dialogues locaux

- [x] Intégrer Ollama et `mod-ollama-chat` derrière une liste de modèles autorisés et une écoute locale.
- [x] Faire décider RealmBox depuis CanIRun, puis télécharger le modèle sur demande avec sa taille annoncée avant confirmation et son manifeste vérifié par digest.
- [x] Afficher l’espace disque disponible et refuser le téléchargement si la marge est insuffisante.
- [ ] Tester en jeu une conversation complète avec un bot et mesurer RAM, latence et débit.
- [x] Permettre d’activer et désactiver les dialogues sans réinstaller le royaume, monde fermé pour recharger le module.
- [x] Permettre de changer le niveau de bavardage, monde arrêté, avec trois presets bornés.
- [x] Configurer un mode sans réseau pendant le jeu et rendre ce statut visible.

## Choix du client

- **Windows x64** : le `Wow.exe` 3.3.5a d’une copie compatible peut être utilisé directement. OpenWoW doit rester une option, pas une obligation.
- **macOS Apple Silicon moderne** : les données du client Windows sont utilisables, mais son exécutable ne tourne pas nativement. OpenWoW arm64 est le chemin natif retenu ; la solution indiquée par ChromieCraft est sinon une machine virtuelle Windows.
- **Linux** : le client Windows peut fonctionner via Wine ; OpenWoW est aussi conçu pour Linux. RealmBox Linux reste à implémenter et tester.
- **Mac Intel** : hors parcours géré tant qu’un binaire OpenWoW x86-64 distribué et vérifié n’est pas disponible.

Le téléchargement ChromieCraft fournit une copie complète configurée pour leur serveur. RealmBox peut reconnaître son dossier `Data` et, sur Windows, son `Wow.exe`. Il ne remplace pas OpenWoW pour une exécution native sur Apple Silicon. RealmBox ne redistribue pas cette copie propriétaire.
