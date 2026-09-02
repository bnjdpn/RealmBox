# RealmBox Companions

L’addon expose uniquement des boutons associés à des commandes constantes. Aucune saisie utilisateur n’est concaténée à une commande Playerbots.

`Former mon équipe` envoie successivement quatre commandes bornées `addclass` pour obtenir un paladin, un prêtre, un mage et un chasseur du niveau du joueur. Playerbots crée le groupe et invoque les bots près du joueur. Les boutons suivants envoient `follow`, `attack`, `stay`, `summon`, `cooldowns on` ou `leave` sur le canal du groupe.

Preuve réelle du 2 septembre 2026 : OpenWoW a chargé l’addon, les quatre commandes `addclass` ont créé un groupe complet autour du joueur, les cadres de groupe étaient visibles et la base locale a confirmé les cinq membres. Les actions de combat et le changement de composition restent à tester séparément.

Le dialogue `mod-ollama-chat` n’est pas géré par ce panneau. Il reste désactivable depuis RealmBox et doit être prouvé avec un modèle local avant d’ajouter un contrôle en jeu.
