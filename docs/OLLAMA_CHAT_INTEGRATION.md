# Intégration mod-ollama-chat

Le commit `a9d14b0...` contient des réglages vérifiés comme `OllamaChat.MaxConcurrentQueries`, `NumPredict`, `NumCtx`, `ThinkMode`, l’historique, les canaux et les chatter. Les presets RealmBox utilisent uniquement ces clés réelles, complétées par un patch lié explicitement à ce commit immuable.

## Runtime local

Le module est compilé dans les images serveur épinglées mais reste désactivé par configuration. Ollama 0.33.2 n’est téléchargé qu’après confirmation depuis sa release officielle et est refusé si son SHA-256 diffère. RealmBox soumet son allowlist courte à CanIRun, filtre les résultats confortables selon son budget mémoire, puis décide automatiquement du meilleur rapport vitesse estimée/taille de téléchargement parmi les modèles 3B+ ; le 1B est un repli. La taille officielle est affichée avant téléchargement. Après le `pull`, le manifeste local doit correspondre au digest immuable du catalogue. L’inférence s’effectue ensuite sur `127.0.0.1:11435` avec les fonctions cloud coupées.

La vue Dialogues peut installer, activer ou désactiver ce runtime après l’installation du royaume. L’activation et la désactivation exigent que le monde soit fermé. Quand l’IA et son modèle sont déjà actifs, les modes de discussion peuvent être appliqués à chaud par la commande bornée `ollama reload`. Une désactivation conserve les fichiers du modèle dans le répertoire géré ; sa réactivation privilégie explicitement ce modèle déjà installé et ne dépend pas d’une nouvelle réponse CanIRun.

## Modes de discussion

Le mode reste indépendant de la population, de la présence des bots et du comportement **Escorte / Garde / Libres** de l’équipe.

| Libellé joueur | Valeur interne | Comportement configuré |
|---|---|---|
| **Direct** | `quiet` | aucun bavardage aléatoire ou événementiel et aucune réponse bot-à-bot ; seuls les messages joueur éligibles peuvent déclencher une réponse |
| **Immersif** | `balanced` | bavardage aléatoire à 20 %, événement commenté par un bot à 8 % ou par le bot concerné à 1 %, réponse bot-à-bot en dire à 20 % et en groupe à 50 %, intervalle aléatoire de 90 à 180 s ; plafond ambiant de 2 envois par minute et par portée, 4 globalement |
| **Vivant** | `lively` | bavardage aléatoire à 35 %, événement commenté par un bot à 10 % ou par le bot concerné à 2 %, réponse bot-à-bot en dire à 35 % et en groupe à 100 %, intervalle aléatoire de 30 à 90 s ; plafond ambiant de 4 envois par minute et par portée, 6 globalement |

Un seul bot est choisi par message et un seul worker LLM traite une file de quatre demandes maximum. Les combats, canaux personnalisés et guildes restent exclus. Les pourcentages sont des chances de soumission avant les plafonds de file et de débit : ils ne décrivent ni un nombre garanti de messages ni une fréquence observée.

Tous les types de requêtes partagent aussi un coupe-circuit local. Trois échecs consécutifs suspendent les nouveaux appels pendant cinq secondes. Une seule requête sert ensuite de sonde ; si elle échoue, les fenêtres passent à 10, 20, 40 puis 60 secondes au maximum. Une réussite ou un rechargement de configuration réinitialise la politique. Une génération empêche une réponse ancienne de rouvrir ou refermer un circuit reconfiguré. Cette politique ne crée ni nouvelle tentative, ni sommeil, ni thread : le repli reste silencieux pour le monde.

Le prompt transmis au petit modèle reste volontairement minimal. Les profils RealmBox désactivent l’historique de discussion, la mémoire et les relations évolutives, le RAG, le suivi de sentiment ainsi que les emotes générées. Aucun de ces éléments n’est injecté ou persisté pour simuler une personnalité durable. La température est fixée à zéro.

Pour une réponse directe, le prompt demande au modèle de répondre dans la langue du dernier message joueur. Pour un échange ambiant sans ce signal, RealmBox inspecte la locale de la copie client gérée : les prompts aléatoires, leurs variations et les prompts événementiels sont français avec `frFR`, anglais avec les autres locales prises en charge. Le test Rust couvre la sélection et la génération des deux variantes ; le respect linguistique par le modèle reste à qualifier dans OpenWoW.

Ces réglages décrivent des probabilités et des plafonds, pas une promesse de réponse ni un rythme observé. **Direct** limite la nature des échanges mais chaque message joueur n’est pas garanti. **Immersif** et **Vivant** autorisent un rebond borné ; ils ne créent pas une conversation illimitée.

## Priorité aux demandes humaines

La chance de réponse à un message joueur éligible est de 100 % dans les trois modes. Sous verrou de file, ces tâches sont insérées en FIFO avant les tâches ambiantes déjà en attente. Sur une profondeur maximale de quatre, le bavardage ambiant ne peut occuper que trois emplacements : le dernier reste réservé à une future demande humaine. Si une ancienne tâche ambiante occupe déjà une file pleine, une demande humaine peut l’évincer.

Cette chance à 100 % et cette priorité ne constituent pas une garantie de réponse. Une file pleine uniquement de demandes humaines refuse la nouvelle demande ; un échec Ollama ou une destination qui n’existe plus au retour du modèle empêchent toujours la livraison. Les réponses humaines contournent le budget de bavardage et le filtre de répétition afin que l’ambiance ne consomme pas leur quota, mais elles ne contournent ni la file maximale, ni le worker unique, ni la disponibilité du modèle et du monde.

## Portées de conversation

Les clés du gouverneur pour **Party** et **Raid** incluent désormais le GUID du groupe dans les chemins de réponse, de bavardage aléatoire et d’événement. Deux groupes distincts ne partagent donc plus leur cooldown, leur historique de répétition ni leur plafond par portée. Les canaux dire restent isolés par zone, et le plafond global des échanges ambiants reste réellement global.

`RandomChatterMaxBotsPerPlayer` est compté par portée pendant chaque passe : une soumission refusée par la file n’est pas comptée comme un message accepté. Un changement de mode à chaud recharge d’abord la configuration, retire les tâches ambiantes encore en file, invalide par génération les résultats ambiants déjà en vol et réinitialise leurs délais. Les demandes humaines en attente ou en vol sont conservées, ainsi que les limites de profondeur, de concurrence et de débit.

## Niveau de preuve et risques

Les garanties de priorité, de réservation, d’isolation et de sélection FR/EN ci-dessus sont des garanties structurelles du patch source, de la configuration générée et de leurs contrôles automatisés. Elles ne prouvent ni la compatibilité d’un build serveur combiné, ni la latence du modèle, ni la cadence perçue, ni la qualité et la langue des répliques dans le vrai client.

Le coupe-circuit est vérifié en compilant et exécutant la politique C++ exacte extraite du patch, et le patch s’applique proprement au commit upstream épinglé. Le worldserver complet et une nouvelle image serveur n’ont cependant pas été reconstruits dans ce lot : cette preuve ciblée n’est pas une preuve de runtime.

Une preuve réelle antérieure avait obtenu une réponse anglaise puis une réponse française avec le preset direct de l’époque. Elle ne qualifie pas les nouveaux modes **Immersif** et **Vivant**, les échanges entre bots, l’isolation simultanée de plusieurs groupes ni l’absence de flood. Un build combiné, un chargement worldserver puis un parcours OpenWoW réel restent nécessaires.

Le README du commit upstream nomme encore les anciens dépôts `liyunfan1223`, tandis que RealmBox épingle les continuations `mod-playerbots`. Les ordres de gameplay restent séparés de la conversation et bornés par allowlist.
