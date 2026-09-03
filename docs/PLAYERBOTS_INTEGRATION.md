# Intégration Playerbots

Le README et le guide officiels du module imposent encore le fork `mod-playerbots/azerothcore-wotlk`, branche `Playerbot`. Les pins actuels sont dans `third-party.lock.toml`. La documentation upstream avertit qu’elle peut être obsolète ; RealmBox ne génère que des clés réellement présentes dans `conf/playerbots.conf.dist` au commit épinglé.

## Réglages indépendants

L’expérience ne repose plus sur un unique preset dense. Elle expose quatre réglages indépendants :

1. la **population** demande 5, 25, 50, 100 ou 150 aventuriers autonomes dans le monde ;
2. la **présence** choisit entre **Dispersés**, **Naturelle** et **Toujours proches** sans modifier ce total ;
3. le **comportement de l’équipe** choisit **Escorte**, **Garde** ou **Libres** uniquement pour les quatre membres formés par l’addon ;
4. le **mode de discussion** choisit **Direct**, **Immersif** ou **Vivant**, sans pouvoir sur les ordres de gameplay.

Changer la présence n’ajoute donc pas de bots, et choisir **Escorte** ne rapproche pas les bots autonomes du monde.

## Population

RealmBox propose au joueur 5, 25, 50, 100 ou 150 bots, puis borne la valeur à partir de la mémoire visible par Docker : 5 sous 12 Gio, 50 sous 20 Gio, 100 sous 28 Gio et 150 au-delà. Une valeur inconnue retombe aussi à 5. Les guildes aléatoires sont désactivées pour éviter leur coût mémoire.

Le launcher conserve séparément la population souhaitée et la valeur sûre réellement appliquée. Lorsque le worldserver géré tourne, il recharge la configuration avec la passerelle bornée `reload config`, `playerbots rndbot reload`, puis `playerbots rndbot update`. Monde arrêté, le même choix est enregistré pour la prochaine partie. Aucune console serveur générique n’est exposée.

## Présence dans le monde

Les trois profils alimentent à la fois `mod-playerbots` et `mod-realmbox-presence` :

| Profil | Politique générée | Intention joueur |
|---|---|---|
| **Dispersés** | aucun placement RealmBox ; cible 0 ; voyage natif Playerbots privilégié ; l’échéance des bots placés et encore suivis par l’instance courante peut être raccourcie à 60 s lorsqu’ils sont autonomes, sûrs et hors de vue | laisser les aventuriers vivre dans l’ensemble du monde sans rapprochement artificiel |
| **Naturelle** — recommandé | cible globale 30 %, 3 à 15 bots par joueur réel ; passe toutes les 2 s ; placement à 90–180 m dans un rayon compté de 220 m ; nouveau placement du même bot après au moins 300 s ; retour au voyage natif après 600 s | faire passer quelques aventuriers de même faction dans la zone, puis les laisser repartir |
| **Toujours proches** | cible globale 60 %, 4 à 30 bots par joueur réel ; passe chaque seconde ; placement à 50–110 m dans un rayon compté de 150 m ; nouveau placement du même bot après au moins 60 s ; retour au voyage natif après 900 s | maintenir une présence plus dense et plus visible |

Ces nombres décrivent la configuration générée, pas une cadence mesurée dans OpenWoW. La cible globale est partagée entre les joueurs réels et bornée aux bots disponibles de même faction. Un seul déplacement est autorisé par passe. Le module exclut notamment les bots groupés ou maîtrisés, les instances et phases incompatibles, les combats, champs de bataille, files LFG, vols, transports et véhicules. Il ne fait pas disparaître un bot encore visible par un joueur réel.

Une installation neuve choisit **Naturelle**. Une installation antérieure à 0.4.0 qui ne possède pas encore `world-preferences.json`, ou dont la préférence de présence est absente, retombe sur **Toujours proches** afin de préserver son comportement antérieur. Dès que le joueur enregistre un choix, celui-ci devient la préférence durable.

## Retour à l’autonomie

Un bot placé près du joueur n’est pas immobilisé. Après le placement, `mod-realmbox-presence` appelle l’API Playerbots `ScheduleTeleport` avec le délai borné du profil au lieu d’écrire une valeur générique valable pendant toute la durée maximale en monde. Playerbots reprend ensuite son propre cycle de voyage et revérifie ses contraintes au moment prévu.

Quand le profil **Dispersés** désactive le placement RealmBox, le module peut raccourcir, sans téléportation immédiate, l’échéance des bots qu’il a lui-même placés et qu’il suit encore en mémoire dans le processus `worldserver` courant. Il attend qu’ils soient autonomes, sûrs, hors groupe et hors de vue, et ne remplace jamais une échéance déjà plus proche.

Cette propriété est volontairement conservatrice : Playerbots n’expose pas de métadonnée publique permettant de distinguer avec certitude un ancien événement RealmBox d’un événement de voyage natif de même forme. Après un redémarrage du `worldserver`, les marques en mémoire sont donc perdues et RealmBox laisse les événements non marqués intacts au lieu d’appliquer une heuristique risquée.

Un bot qui quitte l’équipe reçoit en plus cinq minutes de grâce pendant lesquelles la présence RealmBox ne peut pas le reprendre. Cette grâce et le délai `ScheduleTeleport` répondent à deux besoins distincts : ne pas recapturer immédiatement un compagnon libéré, puis rendre tout bot déplacé au scheduler natif.

## Comportement de l’équipe en jeu

L’addon ne déplace pas arbitrairement les bots autonomes. Il ne pilote que les membres du groupe avec trois choix explicites et bornés :

| Choix | Commande constante | Effet demandé |
|---|---|---|
| **Escorte** | `nc +follow,-stay,-new rpg,-grind` | suivre le joueur |
| **Garde** | `nc +stay,-follow,-new rpg,-grind` | tenir la position |
| **Libres** | `nc +new rpg,+grind,-follow,-stay` | reprendre les activités RPG autonomes |

La sélection affichée est la dernière préférence envoyée, pas un accusé de réception du serveur. Après **Former mon équipe**, l’addon attend que les quatre emplacements soient connectés et que la composition soit stable pendant 1,5 seconde avant de réappliquer la préférence. Après reconnexion, il attend de même un groupe connecté et stable avant une unique réapplication. Cette attente expire après 30 secondes : une équipe formée bien plus tard ne reçoit donc pas un ordre différé destiné à une ancienne reconnexion. Les ordres ponctuels **Me suivre** et **Attendre ici** ne remplacent pas silencieusement cette préférence. **Libérer l’équipe** envoie d’abord **Libres**, puis `leave`.

## Niveau de preuve

Le 2 septembre 2026, avant ce rework, un parcours réel avait maintenu 50 bots avec 15,8 Gio accordés à Docker, un `worldserver` autour de 5,2 Gio et quatre compagnons invoqués. Cette mesure historique valide le palier 50 sur ce Mac, pas les profils de présence actuels ni les paliers 100 et 150.

Le rework actuel est couvert au niveau automatisé par les tests Rust de génération et de préférences, les tests C++ de politique de présence et le harnais Fengari exécutant le vrai Lua de l’addon. La cadence visuelle des trois profils, le retour effectif aux activités natives, les boutons **Escorte / Garde / Libres** et leur persistance restent à observer dans un nouveau parcours OpenWoW réel.
