# RealmBox Companions

## Parcours joueur

L’addon ouvre son panneau lors de la première utilisation, puis conserve sa visibilité. Le bouton de fermeture, la touche Échap, l’icône de minimap et les commandes `/realmbox` ou `/rb` affichent ou masquent le même panneau. L’icône se déplace autour de la minimap ; sa position et celle du panneau sont enregistrées dans `RealmBoxCompanionsDB`.

La langue suit le client au premier lancement, puis peut être changée avec le bouton `FR`/`EN` ou `/realmbox fr` et `/realmbox en`. Ce choix est lui aussi conservé.

Le panneau affiche les membres connectés que l’API du client voit dans le groupe, leurs classes et le nombre de membres hors ligne. Cette information décrit le groupe WoW ; l’API ne permet pas à l’addon de certifier qu’un membre est un bot. Les ordres de groupe sont désactivés quand le groupe est vide. `Attaquer` exige en plus une cible ennemie vivante, et les infobulles expliquent le prérequis manquant.

## Commandes bornées

`Former mon équipe` envoie successivement quatre commandes constantes `addclass` pour obtenir un paladin, un prêtre, un mage et un chasseur du niveau du joueur. Les classes déjà connectées sont conservées et les membres hors ligne sont d’abord retirés. Playerbots crée ensuite le groupe et invoque les bots près du joueur.

Les autres boutons n’acceptent aucune saisie joueur et envoient uniquement les valeurs constantes suivantes sur le canal du groupe :

| Action | Commande Playerbots |
|---|---|
| Me suivre | `follow` |
| Attaquer | `attack` |
| Attendre ici | `stay` |
| Se regrouper | `summon` |
| Capacités fortes activées | `co +boost` |
| Capacités fortes limitées | `co -boost` |
| Libérer l’équipe | `leave` |

Avant toute action, le bouton des capacités affiche `serveur` plutôt que d’inventer l’état initial de Playerbots. Il conserve ensuite la dernière préférence demandée. Playerbots ne renvoie pas à l’addon un état structuré permettant d’en faire un accusé de réception ; l’interface ne présente donc jamais cette valeur comme une confirmation serveur. Les commandes `co +boost` et `co -boost` correspondent au mécanisme de stratégie de combat présent dans le commit Playerbots épinglé par RealmBox. L’ancienne valeur `cooldowns on` n’était pas une commande prise en charge par ce commit.

## Preuves et limites

Le harnais Node exécute le vrai Lua avec Fengari et une API WoW simulée. Cinq tests couvrent les états du panneau, les positions, les langues, la composition, les prérequis et les commandes bornées. Le XML est analysé séparément et le manifeste cible toujours l’interface `30300`.

Preuve réelle antérieure du 2 septembre 2026 : OpenWoW a chargé l’addon, les quatre commandes `addclass` ont créé un groupe complet autour du joueur, les cadres de groupe étaient visibles et la base locale a confirmé les cinq membres. La nouvelle icône, la persistance, le bilingue, les contrôles contextuels et `co ±boost` restent à requalifier visuellement dans OpenWoW.

Le dialogue `mod-ollama-chat` n’est pas géré par ce panneau. Il reste désactivable depuis RealmBox et doit être prouvé avec un modèle local avant d’ajouter un contrôle en jeu.
