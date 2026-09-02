# État factuel

Mis à jour le 2 septembre 2026 sur macOS 26.6.2 arm64.

Décision actuelle : **GO pour poursuivre et faire essayer l'installation sur une copie utilisateur ; NO-GO pour affirmer que le parcours réel complet est validé ou distribuer une release.**

| Fonction | État | Preuve actuelle |
|---|---|---|
| Lanceur inspiré de l'ère Wrath | implémenté | composition calée sur le launcher 3.3.5a fourni ; illustration fantasy originale, aucune ressource Blizzard ; QA Playwright à 1200 px |
| Premier lancement | implémenté, non exécuté de bout en bout | commandes Tauri réelles, tests Rust des frontières et tests UI ; aucune copie 3.3.5a disponible sur la machine de développement |
| Client OpenWoW | artefact vérifié séparément | release officielle 0.1.2 macOS arm64, SHA-256 `832cb82fd853417ec64d8fd1a84cb8c6a91a57399fd4b87fb2e810a35b03ed18`, signature ad hoc valide |
| Serveur AzerothCore Playerbots | source épinglée, build installateur non exécuté | fork `47960183...`, module `2f7d9f77...`; un ancien spike natif compilait, ce qui ne prouve pas le build Docker actuel |
| Données serveur | extraction locale implémentée, non exécutée | volume Docker géré ; `Data` utilisateur monté en lecture seule ; aucun téléchargement de données extraites |
| MySQL | orchestration implémentée, non exécutée dans ce parcours | image multiarchitecture verrouillée par digest ; port `127.0.0.1:3307` |
| Compte joueur local | implémenté, non testé contre une base réelle | calcul SRP6 aligné sur la source épinglée et vecteur de régression ; création idempotente `REALMBOX / REALMBOX` |
| Playerbots à la demande | configuration implémentée | 50 bots quand activé, zéro et autologin coupé sinon ; comportement en jeu non testé |
| Conseil CanIRun | API réelle intégrée | requête limitée au CPU, aux cœurs et à la RAM ; allowlist et budget RealmBox testés ; sur le Mac de développement, `qwen3:8b` Q4 est classé confortable, note S, estimation 77 tok/s |
| Dialogues IA locaux | installateur et cycle de vie implémentés, parcours complet non exécuté | `mod-ollama-chat` au commit `a9d14b0...` ; Ollama 0.33.2 vérifié par SHA-256/signature puis démarré réellement sur `127.0.0.1:11435`, endpoint `/api/version` lu et cloud confirmé coupé ; modèle non téléchargé et dialogue en jeu non testé |
| Second lancement automatique | implémenté et testé avec effets factices | état persisté → base → extraction idempotente → migrations → serveurs → client ; aucune preuve réelle complète |
| Arrêt | implémenté | bouton explicite et supervision du PID OpenWoW ; client → services Docker → Ollama, avec tentative de tous les nettoyages même si l'un échoue ; transition automatique testée avec effets factices, pas de smoke complet réel |
| Bundle Tauri actuel | buildé et lancé | `RealmBox.app` arm64, addon embarqué, signature ad hoc complète vérifiée ; la fenêtre native a reçu en direct la recommandation CanIRun `qwen3:8b` ; exécutable SHA-256 `9739c1525050405b159548e9217afede8947cdfed8f092e1bf0ecc6bbdc5bfee` ; pas de notarisation |
| Windows / Mac Intel | non commencé pour ce parcours | aucune exécution |
| Signature de distribution / notarisation | bloqué | certificats absents |

## Ce qui manque pour déclarer le parcours fonctionnel

- sélectionner une copie utilisateur 3.3.5a valide ;
- laisser le build et les extracteurs terminer ;
- relancer RealmBox et constater les ports 3724/8085, l'ouverture du client et la connexion avec le compte local ;
- créer un personnage, entrer dans le monde et observer les Playerbots activés/désactivés ;
- avec l'option IA, laisser télécharger le modèle recommandé puis observer une réponse de `mod-ollama-chat` tout en contrôlant la mémoire et la latence ;
- vérifier l'arrêt puis une nouvelle reprise après redémarrage de la machine.

Les tests automatisés, le build d'une dépendance ou l'ouverture de la fenêtre ne remplacent pas cette preuve réelle.
