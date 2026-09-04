# RealmBox Companions

## Parcours joueur

L’addon ouvre son panneau lors de la première utilisation, puis conserve sa visibilité. Le bouton de fermeture, la touche Échap, l’icône de minimap et les commandes `/realmbox` ou `/rb` affichent ou masquent le même panneau. L’icône se déplace autour de la minimap ; sa position et celle du panneau sont enregistrées dans `RealmBoxCompanionsDB`.

La langue suit le client au premier lancement, puis peut être changée avec le bouton `FR`/`EN` ou `/realmbox fr` et `/realmbox en`. Ce choix est lui aussi conservé.

Le panneau affiche les membres connectés que l’API du client voit dans le groupe, leurs classes et le nombre de membres hors ligne. Cette information décrit le groupe WoW ; l’API ne permet pas à l’addon de certifier qu’un membre est un bot. Les ordres de groupe sont désactivés quand le groupe est vide. `Attaquer` exige en plus une cible ennemie vivante, et les infobulles expliquent le prérequis manquant.

## Équipes et compagnon principal persistants

Trois préréglages fermés forment une équipe de **cinq joueurs au total : le joueur et quatre emplacements**. Le choix est conservé entre les sessions :

| Préréglage FR / EN | Intention des quatre emplacements |
|---|---|
| Polyvalente / Versatile | paladin tank, prêtre soin, mage dégâts, chasseur dégâts |
| Arcanes / Arcane | paladin tank, prêtre soin, deux mages dégâts |
| Pistage / Tracking | paladin tank, prêtre soin, deux chasseurs dégâts |

Les rôles affichés sont une **intention de composition**, pas une spécialisation imposée ni un état confirmé : l’addon n’envoie aucune commande de rôle non attestée. Les quatre commandes `addclass` déjà utilisées par RealmBox constituent la liste exhaustive des classes demandables. Les variantes avec doublons sont couvertes par le harnais, pas encore par un parcours OpenWoW réel.

Chaque préréglage conserve séparément les dernières préférences de comportement et de capacités envoyées au groupe. Sélectionner un préréglage restaure ces préférences à l’écran sans envoyer de commande. Les anciens réglages globaux sont repris dans le préréglage actif lors de la première ouverture de cette version.

`Former mon équipe` remplit uniquement les places réellement libres. Les membres existants, connectés ou hors ligne, occupent toujours leur place ; **aucun membre n’est expulsé**. L’addon n’utilise plus `UninviteUnit`, car le client ne peut pas distinguer un humain d’un bot de manière fiable. La file s’arrête si le groupe atteint cinq joueurs et expire après 30 secondes pour ne pas adresser de demandes tardives à un autre groupe.

Après une formation, lorsque l’API observe les quatre membres connectés dans le délai de 30 secondes, leurs noms et classes sont mémorisés sous le préréglage. Il s’agit de noms **observés**, pas d’une certification de bots ni d’une promesse de retrouver les mêmes personnages : aucune commande de rappel par nom n’est inventée.

Le joueur peut cibler un membre du groupe puis choisir `Définir la cible principale` / `Set targeted companion as primary`. Son nom reste enregistré lorsqu’il est absent et l’interface l’indique. Ce choix local ne recrute personne, ne change pas le chef de groupe et n’envoie aucune commande serveur.

### Rappel nominatif : non exposé après audit du code épinglé

Le roster mémorisé reste un **historique local d’observation**, pas une escouade identique rappelable. L’audit du 3 septembre 2026 a vérifié la commande exacte `.playerbots bot add NOM` au commit `2f7d9f774987d0157c6a0d0cc08c40bec3db3945`. Elle existe, mais ne fournit pas le contrat nécessaire à un bouton sûr de rappel d’aventuriers autonomes :

| Contrôle observé | Conséquence pour RealmBox |
|---|---|
| La commande `bot` est accessible à `SEC_PLAYER`, puis `add` résout un nom de personnage. | La syntaxe est attestée, mais cette accessibilité ne certifie pas que le nom désigne un bot rappelable. [Déclaration](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Script/PlayerbotCommandScript.cpp#L28-L41), [résolution du nom](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Bot/PlayerbotMgr.cpp#L1283-L1297) |
| `add` refuse un personnage déjà connecté avec `player already logged in`. | Un bot libéré mais encore autonome dans le monde n’est pas réinvité par cette commande. [Gestionnaire](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Bot/PlayerbotMgr.cpp#L682-L704) |
| `AddPlayerBot` autorise un personnage hors ligne seulement via compte propre, guilde autorisée, compte lié ou classification `AddClass`. `IsAddclassBot` reconnaît notamment les comptes de type 2. | Les RNDbots ordinaires de type 1 ne sont pas rappelables de manière générale par leur seul nom. Le client ne connaît ni cette classification ni les droits actuels ; un nom observé peut aussi être un humain ou un personnage de compte/guilde. [Droits](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Bot/PlayerbotMgr.cpp#L83-L145), [classification AddClass](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Bot/RandomPlayerbotMgr.cpp#L2154-L2181) |
| `addclass` choisit son cache par faction ; `add NOM` n’applique pas ce même filtrage dans son gestionnaire. Les contrôles de sécurité des commandes de chat refusent une faction opposée, mais ne constituent pas une validation préalable du rappel. | Une ancienne appartenance au groupe ne prouve pas une compatibilité de faction actuelle. Aucun rappel inter-faction n’est supposé sûr. [Filtre addclass](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Bot/PlayerbotMgr.cpp#L1160-L1176), [sécurité du chat](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Mgr/Security/PlayerbotSecurity.cpp#L42-L58) |
| Le login est asynchrone ; l’invitation peut convertir un groupe complet en raid. Le résultat `ok` est renvoyé avant la confirmation effective de connexion et d’appartenance. | Un contrôle de places dans le seul addon ne garantit pas atomiquement un groupe de cinq. Aucun succès de rappel ne peut être inventé à partir du retour texte. [Retour add](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Bot/PlayerbotMgr.cpp#L682-L704), [invitation après login](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Bot/PlayerbotMgr.cpp#L544-L568), [conversion en raid](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Script/WorldThr/PlayerbotOperations.h#L28-L94) |

Il n’y a donc **ni bouton de rappel nominatif, ni remplacement silencieux, ni commande `add NOM` générée depuis les noms mémorisés**. Une implémentation sûre demande d’abord une passerelle serveur typée qui vérifie atomiquement l’identité bot et son type, les droits du joueur, la faction, la disponibilité, l’absence de combat, l’absence d’un autre maître/groupe et la capacité du groupe sans conversion en raid. Elle devra renvoyer un résultat par bot — rejoint, absent, occupé, interdit, faction incompatible, groupe complet — puis un état relu après l’invitation. Cette passerelle n’est pas implémentée par ce lot.

## Portée et aperçu

Les boutons **Groupe / Party** et **Cible / Target** rendent la portée explicite et persistante. L’aperçu indique la prochaine commande exacte et son destinataire ; les actions locales annoncent qu’aucune commande serveur ne sera envoyée.

- **Groupe** : les ordres existants utilisent `PARTY` ; la formation utilise les commandes constantes `addclass` sur `SAY`.
- **Cible** : seuls `follow`, `stay`, les trois stratégies `nc` et `co ±boost` sont permis. Le destinataire doit être à la fois le compagnon principal enregistré, la cible actuelle et un membre connecté du groupe courant. Le nom est relu dans l’API du client ; aucun nom saisi ni ancien nom isolé ne sert de destinataire.
- **Cible** ne permet ni attaque, ni invocation, ni libération, ni formation. Les contrôles expliquent en FR/EN qu’il faut passer à la portée groupe.

Les préférences de comportement et de capacités du compagnon principal sont distinctes de celles du groupe. Un ordre ciblé ne remplace donc pas la préférence de toute l’équipe. Changer de compagnon principal remet ses préférences ciblées à l’état inconnu ; l’addon ne prétend pas connaître ses réglages serveur.

Le transport ciblé a été vérifié dans le code primaire de Playerbots au commit immuable `2f7d9f774987d0157c6a0d0cc08c40bec3db3945` : [le gestionnaire WHISPER ne transmet qu’à l’AI du destinataire](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Script/Playerbots.cpp#L171-L185), [les raccourcis follow/stay](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Ai/Base/Strategy/ChatCommandHandlerStrategy.cpp#L51-L52) et [les stratégies co/nc](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Ai/Base/Strategy/ChatCommandHandlerStrategy.cpp#L109-L110) sont enregistrés. [Les contrôles de sécurité Playerbots restent applicables](https://github.com/mod-playerbots/mod-playerbots/blob/2f7d9f774987d0157c6a0d0cc08c40bec3db3945/src/Bot/PlayerbotAI.cpp#L953-L1020). Cette lecture prouve le chemin source, pas une exécution réelle dans le bundle installé. Un humain ne possède pas de `PlayerbotAI` : le module n’exécute pas d’ordre pour lui, même si le message privé peut lui être visible.

## Commandes bornées

`Former mon équipe` envoie successivement au plus quatre commandes constantes `addclass`, selon le préréglage et les classes déjà présentes. Les places hors ligne sont conservées. Playerbots prend ensuite en charge la création du groupe et l’arrivée des bots ; l’addon ne fournit aucun faux accusé de réception.

Les autres boutons n’acceptent aucune saisie joueur et envoient uniquement les valeurs constantes suivantes. Le sous-ensemble ciblable ci-dessus peut utiliser `WHISPER` ; les autres restent sur le canal du groupe :

| Action | Commande Playerbots |
|---|---|
| Me suivre | `follow` |
| Attaquer | `attack` |
| Attendre ici | `stay` |
| Se regrouper | `summon` |
| Comportement `Escorte` / `Escort` | `nc +follow,-stay,-new rpg,-grind` |
| Comportement `Garde` / `Guard` | `nc +stay,-follow,-new rpg,-grind` |
| Comportement `Libres` / `Free` | `nc +new rpg,+grind,-follow,-stay` |
| Capacités fortes activées | `co +boost` |
| Capacités fortes limitées | `co -boost` |
| Libérer l’équipe | autonomie, puis `leave` |

Les trois comportements sont des choix séparés : aucun cycle ni état caché n’est nécessaire pour demander `Escorte`, `Garde` ou `Libres`. La sélection visuelle conserve uniquement la dernière préférence envoyée dans la portée sélectionnée. Les ordres ponctuels `Me suivre` et `Attendre ici` ne la remplacent pas silencieusement.

`Libérer l’équipe` exige deux clics dans les huit secondes, avec un aperçu des deux commandes et une composition inchangée. Le changement d’un nom du groupe, une nouvelle composition, un changement de portée ou l’entrée en combat invalide la confirmation. La commande envoie d’abord la stratégie libre, puis `leave` : seuls les bots répondant à Playerbots quittent le groupe ; aucune API d’expulsion du client n’est appelée.

Après `Former mon équipe`, l’addon ne réapplique une préférence de groupe enregistrée qu’une fois les quatre emplacements connectés, la file `addclass` terminée et la composition stable pendant 1,5 seconde. Après une reconnexion de l’interface, il attend de même un groupe non vide, entièrement connecté et stable pendant 1,5 seconde, puis n’envoie la préférence qu’une fois. Changer de portée annule cette réapplication. Ces délais évitent d’adresser seulement les premiers bots revenus. Ils constituent une garantie du harnais simulé, pas une mesure du délai réel de connexion de Playerbots.

Si le joueur **ou un membre du groupe** est en combat, les commandes, la formation, la sélection des préréglages, le changement de portée et le choix du compagnon principal sont suspendus. Les commandes sont aussi désactivées si le groupe est entièrement hors ligne. Les timers continuent d’expirer : aucun ordre de formation ou de reconnexion ne survit au-delà de son délai de 30 secondes. Un petit contrôleur sans affichage garde ces garde-fous actifs même lorsque le panneau est fermé.

La formation revalide les places **observées** avant chaque commande, mais ne peut pas annuler un login déjà transmis au serveur. Si un autre joueur remplit le groupe entre cette demande et l’arrivée du bot, le code upstream peut encore convertir le groupe en raid. La limite cinq est donc une intention et un garde-fou client, pas une garantie atomique serveur ; le même endpoint borné décrit ci-dessus est requis pour supprimer cette course.

Avant toute action, le bouton des capacités affiche `serveur` plutôt que d’inventer l’état initial de Playerbots. Il conserve ensuite la dernière préférence demandée. Playerbots ne renvoie pas à l’addon un état structuré permettant d’en faire un accusé de réception ; l’interface parle donc toujours de préférence « envoyée » ou « réappliquée », jamais de comportement confirmé. La réapplication après formation ou reconnexion attend une composition stable pendant 1,5 seconde et expire après 30 secondes, afin de ne jamais envoyer un ordre retardé à une équipe sans rapport. Les commandes `co +boost` et `co -boost` correspondent au mécanisme de stratégie de combat présent dans le commit Playerbots épinglé par RealmBox. L’ancienne valeur `cooldowns on` n’était pas une commande prise en charge par ce commit.

## Preuves et limites

Le harnais Node exécute le vrai Lua avec Fengari et une API WoW simulée. **20 tests passent** : états du panneau, positions, FR/EN, composition, préréglages, noms observés, compagnon principal, persistance séparée des préférences de chaque préréglage et de la cible, aperçu exact, liste fermée `WHISPER`, refus de cible absente/non principale/hors ligne, absence totale d’expulsion humaine et de rappel nominatif non sûr, confirmation et invalidation, combat du joueur ou d’un membre, groupe entièrement hors ligne, saturation/expiration des files, comportements et réapplications différées. `xmllint --noout` valide le XML et le manifeste cible toujours l’interface `30300`.

Preuve réelle antérieure du 2 septembre 2026 : OpenWoW a chargé l’addon, les quatre commandes `addclass` ont créé un groupe complet autour du joueur, les cadres de groupe étaient visibles et la base locale a confirmé les cinq membres. Le panneau élargi, les préréglages, la persistance, le bilingue, la cible privée, les confirmations, le garde-fou combat et `co ±boost` restent à requalifier visuellement et fonctionnellement dans OpenWoW. Aucun joueur ni bot réel n’a été commandé pendant ce lot.

Le dialogue `mod-ollama-chat` n’est pas piloté par l’addon en jeu : son activation et son niveau de bavardage restent dans l’interface RealmBox. Le niveau peut être changé à chaud lorsque le modèle local est actif.
