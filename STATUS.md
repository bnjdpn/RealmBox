# État factuel

Mis à jour le 2 septembre 2026 sur macOS 26.6.2 arm64.

Décision actuelle : **GO pour poursuivre et faire essayer l'installation sur une copie utilisateur ; NO-GO pour affirmer que le parcours réel complet est validé ou distribuer une release.**

| Fonction | État | Preuve actuelle |
|---|---|---|
| Lanceur inspiré de l'ère Wrath | implémenté | UI React sans ressource Blizzard ni image générée ; QA navigateur large et étroite |
| Premier lancement | implémenté, non exécuté de bout en bout | commandes Tauri réelles, tests Rust des frontières et tests UI ; aucune copie 3.3.5a disponible sur la machine de développement |
| Client OpenWoW | artefact vérifié séparément | release officielle 0.1.2 macOS arm64, SHA-256 `832cb82fd853417ec64d8fd1a84cb8c6a91a57399fd4b87fb2e810a35b03ed18`, signature ad hoc valide |
| Serveur AzerothCore Playerbots | source épinglée, build installateur non exécuté | fork `47960183...`, module `2f7d9f77...`; un ancien spike natif compilait, ce qui ne prouve pas le build Docker actuel |
| Données serveur | extraction locale implémentée, non exécutée | volume Docker géré ; `Data` utilisateur monté en lecture seule ; aucun téléchargement de données extraites |
| MySQL | orchestration implémentée, non exécutée dans ce parcours | image multiarchitecture verrouillée par digest ; port `127.0.0.1:3307` |
| Compte joueur local | implémenté, non testé contre une base réelle | calcul SRP6 aligné sur la source épinglée et vecteur de régression ; création idempotente `REALMBOX / REALMBOX` |
| Playerbots à la demande | configuration implémentée | 50 bots quand activé, zéro et autologin coupé sinon ; comportement en jeu non testé |
| Second lancement automatique | implémenté et testé avec effets factices | état persisté → base → extraction idempotente → migrations → serveurs → client ; aucune preuve réelle complète |
| Arrêt | implémenté | processus client appartenant au lanceur puis services Docker ; test de l'interface, pas de smoke réel |
| Bundle Tauri actuel | buildé et lancé | `RealmBox.app` arm64, addon embarqué, signature ad hoc complète vérifiée, exécutable SHA-256 `b8341e3dcc7d9bae4b305590c22e4f7c2aecfb3195ae959684ece5cbfacf1236` ; pas de notarisation |
| Windows / Mac Intel | non commencé pour ce parcours | aucune exécution |
| Signature de distribution / notarisation | bloqué | certificats absents |

## Ce qui manque pour déclarer le parcours fonctionnel

- sélectionner une copie utilisateur 3.3.5a valide ;
- laisser le build et les extracteurs terminer ;
- relancer RealmBox et constater les ports 3724/8085, l'ouverture du client et la connexion avec le compte local ;
- créer un personnage, entrer dans le monde et observer les Playerbots activés/désactivés ;
- vérifier l'arrêt puis une nouvelle reprise après redémarrage de la machine.

Les tests automatisés, le build d'une dépendance ou l'ouverture de la fenêtre ne remplacent pas cette preuve réelle.
