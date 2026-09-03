# Revue de l’écosystème solo et bots — 3 septembre 2026

## Objet et méthode

Cette revue compare les huit sources proposées avec les contraintes de RealmBox : expérience joueur simple en français et en anglais, données de jeu fournies par le joueur, dialogues locaux, dépendances immuables, données de personnages persistantes et mises à jour qui échouent fermées.

Les mentions **vérifié** ci-dessous viennent des pages, dépôts, historiques, licences et releases publics consultés le 3 septembre 2026. Une mention **inférence** est une recommandation RealmBox dérivée de ces faits, pas une propriété annoncée par la source. Aucun binaire, dump, asset de jeu ou code de ces projets n’a été importé.

Un SHA indique seulement l’état public inspecté. Il ne constitue ni une approbation juridique ni une décision d’intégration. Toute future dépendance devra être réévaluée et inscrite dans `third-party.lock.toml` avec sa licence et ses artefacts vérifiables.

## Décision exécutive

RealmBox ne doit remplacer ni son socle AzerothCore/Playerbots, ni son installation atomique, ni son stockage persistant par un repack étudié ici. Le meilleur assemblage est plutôt le suivant :

1. reprendre les bons patrons de contrôle de groupe de SoloCraft, WOW Legends et NPCBots : équipe persistante, rôles explicites, préréglages, portée « groupe ou cible », aperçu exact et confirmation des actions dangereuses ;
2. reprendre les garde-fous d’IA de `mod-ollama-bot-amigo` et de `azerothcore-playerbots-docker-automated` : file bornée, priorité aux joueurs, outils autorisés, validation au dernier moment, budgets de débit, délais, repli silencieux et mémoire limitée ;
3. reprendre les profils solo et le réglage progressif de TrinityCore Single Player et ASP sous forme d’options RealmBox réversibles, jamais sous forme de remplacement opaque de configuration ;
4. différer l’autonomie LLM de déplacement, le guide factuel, la progression par extensions, le hardcore et les grands rosters tant que leur valeur, leur coût et leur récupération ne sont pas prouvés ;
5. rejeter les bases prêtes à l’emploi, données extraites, clients ou repacks, les identifiants par défaut, les connexions IA distantes implicites, les mises à jour de bases sans sauvegarde obligatoire et toute source sans licence compatible.

## Vue comparative

La grille emploie trois décisions : **ADOPTER** signifie intégrer une dépendance telle quelle après verrouillage ; **ADAPTER** signifie réimplémenter un patron dans l’architecture RealmBox ; **REJETER** signifie ne pas intégrer la base, l’artefact ou la pratique. À ce stade, aucune des huit sources n’est assez compatible pour être **ADOPTÉE** comme dépendance directe.

| Source | Signal utile | Maturité publique observée | Décision RealmBox |
| --- | --- | --- | --- |
| `mod-ollama-bot-amigo` | Contrats d’outils LLM, validation et reprise sur erreur | Expérimental, sans release, avertissement « pas pour usage général » | **ADAPTER** l’architecture de sûreté ; **REJETER** le module direct |
| SoloCraft | Formation de groupe extrêmement directe et monde solo vivant | Service ancien et actif ; serveur non open source | **ADAPTER** l’UX ; **REJETER** serveur et PCP opaques |
| TrinityCore Single Player | Catalogue compact de réglages solo | Fork arrêté en avril 2021 | **ADAPTER** les profils ; **REJETER** le fork |
| AzerothCore avec NPCBots | Compagnons stables, rôles, formations et ordres | Fork arrêté en octobre 2023 ; upstream distinct encore actif | **ADAPTER** les concepts ; **REJETER** le changement de cœur |
| SinglePlayerProject | Archive historique de nombreuses variantes solo | Organisation fragmentée, dépôts principaux anciens | **ADAPTER** les enseignements ; **REJETER** comme dépendance |
| Docker Playerbots automatisé | Exploitation, budgets de dialogue, roster, pins | Très jeune, peu de commits, aucune release ni licence racine | **ADAPTER** les patrons ; **REJETER** le code sans licence |
| ASP | Offre solo riche et progression Vanilla/TBC/WotLK | Releases régulières, mais installation/repack risquée | **ADAPTER** le catalogue ; **REJETER** le repack |
| WOW Legends | Meilleure synthèse produit et addon joueur | Projet public récent, app et chaîne de release partiellement opaques | **ADAPTER** l’UX MIT ; **REJETER** repack et cloud par défaut |

## 1. `mod-ollama-bot-amigo`

Sources principales :

- dépôt : <https://github.com/notOrrytrout/mod-ollama-bot-amigo>
- état inspecté : <https://github.com/notOrrytrout/mod-ollama-bot-amigo/commit/9757191d8bf38c753747ab6eaf396754c2ffe18f>
- boucle de contrôle : <https://github.com/notOrrytrout/mod-ollama-bot-amigo/blob/main/src/Script/OllamaBotControlLoop.cpp#L1438>
- validation des actions : <https://github.com/notOrrytrout/mod-ollama-bot-amigo/blob/main/src/Script/OllamaBotControlLoop.cpp#L4643>
- mémoire : <https://github.com/notOrrytrout/mod-ollama-bot-amigo/blob/main/src/Db/BotMemory.cpp#L69>
- licence : <https://github.com/notOrrytrout/mod-ollama-bot-amigo/blob/main/LICENSE>

### Faits vérifiés

- Le module ajoute au-dessus de PlayerbotAI une planification longue et courte, puis exige une seule action structurée par réponse. Le combat reste piloté par PlayerbotAI.
- Les actions sont limitées à une liste connue et leurs arguments sont validés. Pour le déplacement, un `nav_epoch` permet de refuser une décision fondée sur une position périmée.
- Les requêtes LLM sortent de la boucle monde, avec délais, reprise exponentielle et circuit de coupure. L’état est revalidé avant l’exécution.
- La mémoire est bornée ; les actions récemment bloquées reçoivent un délai de refroidissement et les écritures sont lissées.
- Le README classe explicitement le projet comme expérimental et signale notamment des serveurs cassés et une forte consommation CPU. Le dépôt comptait environ soixante commits, une étoile et aucune release ; le dernier commit public inspecté date du 5 février 2026.
- Le fichier `LICENSE` est AGPL-3.0, tandis que le README emploie aussi « GPLv3 ». Cette divergence doit être résolue avant toute réutilisation de code.

### Idées retenues

- **Inférence :** centraliser les demandes d’inférence dans une file bornée partagée, au lieu de créer un travail autonome non plafonné pour chaque bot.
- Exiger un contrat d’outil fermé, des arguments typés et une revalidation sur le thread monde avant toute action.
- Conserver les délais courts et longs, le backoff, le coupe-circuit, le quota d’écritures et l’historique borné.
- Réserver toute autonomie expérimentale à quelques compagnons explicitement choisis, avec désactivation instantanée et télémétrie locale.

### Idées différées

- Planification LLM de quête et de déplacement autonome. Elle demande une preuve longue durée sur blocages, latence, charge serveur et récupération après position périmée.

### Idées rejetées

- Création ou purge de tables de mémoire au démarrage sans migration versionnée ni sauvegarde préalable.
- URL Ollama distante configurable dans le parcours joueur normal, journalisation brute des prompts ou de l’état, et un thread détaché par bot sans plafond global démontré.
- Intégration directe du module tant que son statut expérimental et sa licence déclarée ne sont pas clarifiés.

## 2. SoloCraft

Sources principales :

- actualités : <https://solocraft.org/news>
- site : <https://solocraft.org/>
- forum et changelog : <https://forum.solocraft.org/>
- addon communautaire de composition : <https://github.com/pumpan/FillRaidBots>
- addon communautaire de préréglages : <https://github.com/GabHST/PartyPresets>

### Faits vérifiés

- SoloCraft propose un royaume Vanilla 1.12.1/1.14 orienté solo : expérience x3 avec x1 optionnel, professions accessibles sur un personnage, raids simplifiés, hôtel des ventes alimenté, jeu interfaction et bots pour le monde ou le PvP.
- Son panneau PCP rassemble des commandes de serveur et de bots dans une interface en jeu.
- Le forum restait actif en juillet 2026 et présentait plus de deux cents sujets de changelog, ce qui est un signal d’exploitation durable, pas une preuve de qualité du code.
- Le code du serveur et du PCP ainsi que leur licence n’ont pas été trouvés dans une source officielle publique. Aucun SHA ne peut donc être verrouillé pour ces éléments.
- Les addons communautaires `FillRaidBots` et `PartyPresets` sont publiés sous licence MIT. `FillRaidBots` gère notamment des compositions par rôle, classe et faction, la détection d’instance ou de boss, le remplissage automatique et l’import/export de préréglages. Il ne retire pas de joueur humain et suspend certaines opérations en combat.
- Ces addons ciblent Vanilla 1.12.1 ; ils ne sont pas directement compatibles avec le client WotLK 3.3.5a de RealmBox.

### Idées retenues

- Préréglages nommés de groupe et de raid, avec taille, rôles, classes préférées et mode de remplissage.
- Règles simples : ne jamais évincer un humain, interrompre les mutations de groupe en combat, expliquer pourquoi un rôle ne peut pas être rempli.
- Import/export d’un préréglage ne contenant que des préférences RealmBox, sans données de compte ou de personnage.
- Profils solo réversibles : normal, confortable et accéléré, avec aperçu clair des règles modifiées.

### Idées différées

- Détection de boss et composition automatique spécifique à chaque rencontre, après disponibilité d’une matrice locale et testée pour WotLK.
- BattleBots et remplissage automatique PvP, trop éloignés du parcours solo prioritaire.

### Idées rejetées

- Copie ou port du PCP sans source et licence officielles.
- Installation directe des addons Vanilla dans le client WotLK.
- Reproduction d’un monde hébergé ou de son économie sans preuve d’isolation et de déterminisme local.

## 3. `jjodax/TrinityCore-Single-Player`

Sources principales :

- dépôt : <https://github.com/jjodax/TrinityCore-Single-Player>
- état inspecté : <https://github.com/jjodax/TrinityCore-Single-Player/commit/0e9c464870e7ec4fad9aef3feed4bf03cec04157>
- historique : <https://github.com/jjodax/TrinityCore-Single-Player/commits/master/>

### Faits vérifiés

- Le fork combine des réglages solo, un AHBot, dix-neuf NPCBots, un mode LFG solo et AutoBalance pour les donjons et le monde.
- Les réglages annoncés incluent l’accès aux onze professions, aucune signature requise pour une guilde, la totalité de l’expérience en groupe, l’absence d’exigence de raid et une portée d’aggro réduite.
- Le dépôt est sous GPL-2.0. Il avait une étoile, aucune release et son dernier commit public est celui du 2 avril 2021 ci-dessus.
- Le dernier README remplace une partie de la documentation de construction par des liens externes MediaFire et Warmane. C’est un recul de reproductibilité et de provenance.

### Idées retenues

- Transformer les réglages solo en profils explicites et réversibles, avec valeurs avant/après et restauration du profil RealmBox par défaut.
- Évaluer séparément les options « plusieurs professions », « guilde solo », « groupe sans pénalité » et « accès instance » au lieu d’un unique interrupteur magique.
- Conserver AutoBalance comme piste de laboratoire avec bornes, exclusions et tests de boss.

### Idées différées

- AutoBalance du monde ouvert et LFG entièrement solo, jusqu’à preuve que l’expérience reste lisible et que les rencontres ne sont pas dénaturées.

### Idées rejetées

- Forker ou compiler cette base 2021.
- Télécharger les archives ou clients externes liés par le README.
- Mélanger NPCBots, Playerbots et changements de cœur sans matrice de compatibilité et migrations isolées.

## 4. `zaicopx/AzerothCore-wotlk-with-NPCBots`

Sources principales :

- dépôt : <https://github.com/zaicopx/AzerothCore-wotlk-with-NPCBots>
- état inspecté : <https://github.com/zaicopx/AzerothCore-wotlk-with-NPCBots/commit/730651c4c9322f446f02d18a48e57e3346d53e09>
- historique de la branche : <https://github.com/zaicopx/AzerothCore-wotlk-with-NPCBots/commits/AzerothCore/>
- upstream actuel à surveiller : <https://github.com/trickerer/Trinity-Bots>
- Compose inspecté : <https://github.com/zaicopx/AzerothCore-wotlk-with-NPCBots/blob/AzerothCore/docker-compose.yml#L42>

### Faits vérifiés

- Il s’agit d’un fork complet d’AzerothCore adapté à NPCBots, pas d’un module interchangeable avec le socle RealmBox.
- Les NPCBots sont des créatures compagnons : rôles, équipement, formations, ordres prioritaires, rappel, masquage, LFG, raids, PvP et itinérance.
- La branche étudiée s’arrête au 21 octobre 2023. L’upstream `trickerer/Trinity-Bots` continue d’évoluer ; l’existence de cet upstream ne rend pas le fork zaicopx actuel.
- Le Compose étudié expose MySQL et SOAP, utilise un mot de passe racine trivial, des services root, un worldserver privilégié et des images `master` mutables.
- Le dépôt renvoie aussi vers un repack externe. RealmBox ne doit ni le télécharger ni le redistribuer.
- La provenance est composite : ajouts AzerothCore annoncés sous AGPL-3.0 et héritage MaNGOS/Trinity sous GPL-2.0. Une réutilisation demanderait un audit fichier par fichier.

### Idées retenues

- Un compagnon stable attaché au personnage, rappelable et masquable, avec rôle et formation mémorisés.
- Préréglages de formation, priorité des ordres et retour visuel lorsque l’ordre ne peut pas être appliqué.
- **Inférence :** implémenter ces concepts au-dessus des Playerbots et de l’addon RealmBox existants réduit fortement le risque par rapport à un changement de cœur.

### Idées différées

- Compagnons participant au LFG, aux raids et au PvP ; cela exige des tests de comportement, de persistance et d’équilibrage spécifiques.
- Veille sur `trickerer/Trinity-Bots`, sans dépendance tant qu’un spike isolé n’a pas démontré un avantage inaccessible avec Playerbots.

### Idées rejetées

- Remplacer le cœur RealmBox par ce fork.
- Reprendre son Compose, ses identifiants, ses privilèges, ses ports ou ses images mutables.
- Consommer le repack externe ou ses données de jeu.

## 5. `SinglePlayerProject`

Sources principales :

- organisation : <https://github.com/SinglePlayerProject>
- Playerbots représentatif : <https://github.com/SinglePlayerProject/mod-playerbots-1>
- cœur représentatif : <https://github.com/SinglePlayerProject/AtieshCore>
- ancienne base Trinity : <https://github.com/SinglePlayerProject/TrinityCore-Single>

### Faits vérifiés

- L’organisation publique constitue surtout une archive de forks et variantes couvrant différentes générations de cœurs et de bots.
- Parmi les dépôts représentatifs inspectés, `mod-playerbots-1` a été poussé pour la dernière fois en avril 2023 et annonce MIT, `AtieshCore` en février 2023 et annonce GPL-2.0, tandis que `TrinityCore-Single` remonte à 2019 et annonce GPL-2.0.
- Les licences ne sont pas homogènes et certains dépôts de l’organisation n’en déclarent pas. Une organisation n’a pas de SHA unique ; aucun dépôt SPP n’est retenu comme dépendance à verrouiller.
- L’activité disparate et l’absence d’une distribution moderne unifiée en font une source historique, pas une base de maintenance.

### Idées retenues

- Utiliser l’organisation comme index historique pour comprendre les besoins récurrents du solo : économie, population, compagnons, contrôle de groupe et réglages de progression.
- Comparer une idée avec les upstreams maintenus avant tout spike RealmBox.

### Idées différées

- Aucun port direct. Un dépôt précis pourrait seulement être réévalué si une fonction indispensable n’existe pas dans les upstreams déjà verrouillés.

### Idées rejetées

- Dépendance à un fork ancien au seul motif qu’il agrège davantage de fonctions.
- Copie depuis un dépôt sans licence explicite ou sans historique permettant d’attribuer le code.

## 6. `lathcf/azerothcore-playerbots-docker-automated`

Sources principales :

- dépôt : <https://github.com/lathcf/azerothcore-playerbots-docker-automated>
- état inspecté : <https://github.com/lathcf/azerothcore-playerbots-docker-automated/commit/325bfea3dac71822d5da58bba3375f49ecdf2e6b>
- pins : <https://github.com/lathcf/azerothcore-playerbots-docker-automated/blob/main/repo-pins.txt>
- sauvegarde : <https://github.com/lathcf/azerothcore-playerbots-docker-automated/blob/main/backup.sh>
- restauration : <https://github.com/lathcf/azerothcore-playerbots-docker-automated/blob/main/restore.sh>
- mise à jour : <https://github.com/lathcf/azerothcore-playerbots-docker-automated/blob/main/update.sh>

### Faits vérifiés

- L’overlay automatise AzerothCore Playerbots, des modules et addons, le dialogue Ollama, un guide factuel, des équipes persistantes, ainsi que des expériences arène, Wintergrasp et stratégies de boss.
- Le dialogue réactif a priorité sur l’ambiant, l’ambiant peut être interdit en l’absence d’humain, les messages de commande/addon sont filtrés et les requêtes Ollama ont des limites de longueur, fréquence, débit et historique.
- Le projet désactive le bavardage Playerbots natif afin d’éviter les doubles réponses.
- Le guide factuel utilise un sidecar et un utilisateur MySQL en lecture seule, avec repli silencieux. Le roster conserve les mêmes bots et rôles pour les formats 5/10/25/40 et les aligne sur la progression du joueur.
- `repo-pins.txt` emploie des SHA complets et l’application des patches échoue fermée, deux bons signaux de reproductibilité.
- L’arrêt gracieux prévoit une longue fenêtre pour la sauvegarde des bots.
- La mise à jour lance des migrations sans imposer une sauvegarde vérifiée. La restauration supprime quatre bases avant import ; un import défaillant peut donc laisser les bases absentes. Le contrôle préalable vérifie surtout les noms des archives, pas leur restaurabilité.
- Les sauvegardes embarquent le fichier `.env`, donc des secrets, même si les permissions sont resserrées. Les conversations sont persistées en base.
- Le dépôt avait neuf commits, deux étoiles, trois forks, aucune release et aucun fichier de licence à la racine. En l’absence de licence, les modules personnalisés ne sont pas copiables.

### Idées retenues

- Priorité stricte aux messages humains, emplacement réservé dans la file, suppression possible d’une tâche ambiante et absence d’ambiant sans humain présent.
- Budgets séparés par canal : probabilité de réponse, longueur, messages par minute, délai, historique borné et limite globale de requêtes.
- Guide factuel en lecture seule, fondé sur des données locales autorisées, sans réponse lorsque les sources manquent.
- Roster persistant avec rôles équilibrés et niveau/équipement bornés par le joueur ; population répartie par tranches de niveau.
- Pins complets et patches qui échouent fermés ; arrêt gracieux suffisamment long pour la persistance.

### Idées différées

- Guide factuel et roster 10/25/40, après preuve de performance et expérience joueur sur le roster actuel.
- Arènes, Wintergrasp et scripts de boss, qui augmentent fortement la surface de maintenance.

### Idées rejetées

- Copie des modules personnalisés tant qu’aucune licence ne l’autorise.
- Ses scripts de sauvegarde, restauration et mise à jour dans leur forme actuelle.
- Sauvegarde de secrets avec les données joueurs, téléchargement d’un client ou de données extraites, et service d’inscription web dans le périmètre normal du produit.

## 7. ASP

Sources principales :

- dépôt méta : <https://github.com/kadeshar/ASP>
- état inspecté du dépôt méta : <https://github.com/kadeshar/ASP/commit/e1923a5fd1fc469ad9f1a4bc6f33df2079c8b6be>
- releases : <https://github.com/kadeshar/ASP/releases>
- catalogue des composants : <https://github.com/stars/kadeshar/lists/azerothcore-single-player>

### Faits vérifiés

- ASP regroupe Playerbots, progression Vanilla/TBC/WotLK, hôtel des ventes, hardcore, transmogrification, Ollama, armurerie/statistiques et addons.
- Le dépôt méta a publié vingt releases entre février 2025 et août 2026. La release publique observée la plus récente est `v1.5.0` du 17 août 2026 ; le commit méta inspecté date du 30 août 2026.
- Le dépôt méta ne contient essentiellement que la présentation et la licence ; les composants résident dans plusieurs dépôts. La licence méta est AGPL-3.0, mais les composants annoncent un mélange de GPL-2.0, GPL-3.0 et AGPL-3.0.
- Le parcours Windows repose sur XAMPP, des identifiants par défaut tels que `admin/123456` et `acore/acore`, des archives complètes volumineuses, la copie d’un dossier MySQL et des remplacements de configuration.
- Certaines instructions demandent une sauvegarde manuelle, mais le chemin de mise à jour peut réimporter ou supprimer le monde et aucune preuve uniforme de sauvegarde non écrasante, restaurable et obligatoire n’est fournie.
- Aucun ensemble séparé et stable de checksums n’a été observé pour couvrir la chaîne complète de releases.

### Idées retenues

- Profils de progression Vanilla, TBC puis WotLK présentés comme un choix de monde compréhensible.
- Catalogue d’options solo séparées : économie, rythme, confort, compagnon et difficulté.
- Armurerie locale et statistiques minimales comme piste d’observabilité joueur, sans portail public ni exposition réseau.

### Idées différées

- Progression par extensions, hardcore et transmogrification, car ces fonctions ont des migrations et des conséquences irréversibles à traiter séparément.
- Étude composant par composant, uniquement après sélection d’un besoin précis et verrouillage de son upstream réel.

### Idées rejetées

- Repack ASP comme dépendance ou base de RealmBox.
- XAMPP, comptes par défaut, copie brute du dossier MySQL, écrasement des configurations et mise à jour via archive complète.
- Agrégation de composants dont les licences et SHAs n’ont pas été audités individuellement.

## 8. WOW Legends

Sources principales :

- produit : <https://wow-legends.eu/>
- organisation : <https://github.com/WOWLegendsHQ>
- dépôt Community Edition : <https://github.com/WOWLegendsHQ/wow-legends-community>
- état documentaire inspecté : <https://github.com/WOWLegendsHQ/wow-legends-community/commit/0aa9cccc2512990b01c166a8e17a35343755ac63>
- releases : <https://github.com/WOWLegendsHQ/wow-legends-community/releases>
- addon joueur : <https://github.com/WOWLegendsHQ/wow-legends-player-addon>

### Faits vérifiés

- Le produit propose une installation WotLK 3.3.5a, des Playerbots, un compagnon permanent par personnage, du chat local ou hébergé, une mémoire de conversation, un guide, un mode hardcore, un taux d’expérience par personnage et une économie automatisée.
- L’addon joueur, sous MIT et destiné à WotLK 3.3.5a, distingue les commandes concernant tout le groupe de celles visant le bot ciblé. Il fournit listes fermées, favoris, historique, aperçu de la commande exacte, confirmations de danger et désactivation des modules indisponibles. Il exclut volontairement les commandes GM.
- Les commandes en langage naturel passent par un jeu d’actions déterministes : liste blanche, contrôle du maître et confirmation de groupe pour les actions sensibles.
- Le Guide annonce un routage sur les routes existantes et le Sage des réponses fondées sur les données du serveur ; le compagnon conserve une mémoire sociale et peut être rappelé automatiquement.
- Le dépôt public Community Edition, créé en juin 2026, est surtout un dépôt de documentation et de releases. Il avait deux étoiles et cinq releases lors de l’inspection. La version gratuite observée était `v1.4.2` du 9 août 2026 ; une version `v1.5.3` était annoncée en accès anticipé supporter.
- Les releases distribuent serveur, bases préparées et archives séparées `dbc/maps/vmaps/mmaps/Cameras`. Même avec un fichier `SHA256SUMS.txt`, ces données ne satisfont pas la règle RealmBox « données de jeu fournies par le joueur ».
- La documentation de release attribue AzerothCore à GPL-2.0 et mod-playerbots à AGPL-3.0, tandis que l’écosystème AzerothCore actuel est généralement AGPL-3.0. Cette incohérence et le périmètre réel des sources du repack exigent un audit ; l’app et le portail ne sont pas fournis comme code source public complet.
- Le mode hébergé envoie les messages à DeepSeek selon la politique publiée. Une documentation IA conseille aussi `VerifyTLS=0` dans un scénario. Ces choix sont incompatibles avec le défaut local et sécurisé de RealmBox.
- Le changelog public documente des incidents sévères, dont une commande d’équipement ayant effacé des données de personnage et un butin de zone ayant perdu des objets. C’est un signal utile de risque sur les mutations et les migrations, pas une preuve que les versions actuelles restent affectées.

### Idées retenues

- Un seul compagnon principal par personnage, nommé, rappelable, avec rôle, personnalité locale et mémoire bornée contrôlable.
- Interface addon avec portée explicite « groupe » ou « cible », choix par listes, aperçu exact, favoris/historique locaux et confirmations renforcées pour les actions irréversibles.
- Commandes naturelles converties en intentions énumérées puis validées ; aucune commande GM et aucun texte LLM exécuté directement.
- Guide fondé sur des données locales attestées, qui indique son incertitude ou reste silencieux plutôt que d’inventer.
- Auto-rappel du compagnon comme préférence réversible, jamais pendant une situation incompatible.

### Idées différées

- Mémoire sociale riche, guidage routier, « Sage », Dungeon Clear, hardcore et XP par personnage, chacun dans un lot isolé avec migration, rollback et tests réels.
- Analyse du code source d’une release précise si, et seulement si, son archive source, sa licence, son SHA-256 et la correspondance avec le binaire sont démontrés.

### Idées rejetées

- Repack, bases préparées, données extraites et MySQL portable distribués par les releases.
- IA hébergée par défaut, envoi de conversation à un tiers, clé cloud requise ou désactivation de TLS.
- Identifiants par défaut et comptes GM précréés.
- Copie de l’app, du portail ou du code serveur non publié ; seul le patron UX des addons MIT peut être étudié sous son attribution et sa licence.

## Décisions transversales

### Retenu pour RealmBox

- Une expérience « mon équipe » centrée joueur : équipe persistante, rôles, formation, préréglages et explication immédiate des échecs.
- Une portée de commande toujours visible : tous les compagnons ou compagnon ciblé.
- Des actions sensibles avec aperçu exact, validation typée, confirmation adaptée au risque et annulation quand l’état a changé.
- Des profils solo nommés, réversibles et inspectables plutôt que des fichiers de configuration écrasés.
- Une file de dialogue locale bornée où un humain est prioritaire, avec budgets, délais, repli et mémoire limitée.
- Une source factuelle locale en lecture seule pour un futur guide, séparée de l’action et incapable de muter le monde.
- Des dépendances épinglées par SHA complet et des patches qui échouent fermés.

### Différé

- Autonomie LLM de déplacement ou de quête.
- AutoBalance généralisé, Dungeon Clear, LFG solo et compositions propres aux boss.
- Roster 10/25/40, PvP, arènes et Wintergrasp.
- Progression Vanilla/TBC/WotLK, hardcore, transmogrification et portail local.

Chaque élément différé demande un spike isolé, un budget de performance, des tests de transitions et de récupération, puis une preuve réelle en jeu. Aucun n’entre dans le runtime joueur seulement parce qu’il existe dans un repack.

### Rejeté

- Tout client, DBC, map, vmap, mmap, caméra, dump ou base préparée provenant d’un tiers.
- Tout repack, archive complète opaque ou installateur qui remplace le runtime actif en place.
- Toute mise à jour ou migration sans sauvegarde complète, vérifiée, non écrasante et située hors du runtime remplaçable.
- Suppression de volumes, copie brute d’un répertoire MySQL, import après suppression de bases sans rollback automatique.
- Mots de passe par défaut, services root ou privilégiés, MySQL/SOAP exposés et images mutables.
- IA distante implicite, TLS désactivé, logs de conversations non bornés ou secrets inclus dans les sauvegardes joueur.
- Code sans licence explicite, licence contradictoire ou provenance incapable de relier source, build et artefact distribué.

## Feuille de route RealmBox priorisée

### P0 — verrous réalisés et qualification restante

1. **Protection à la demande — implémentée et couverte automatiquement.** L’interface « Protection » crée un nouveau `manual-backup-*`, impose le dump complet des quatre bases, relit son SHA-256, le conserve hors `runtime-v3` sans écrasement et ne coupe pas une base déjà active. Les tests avec fakes couvrent aussi le démarrage puis l’arrêt du seul service `database`. Ce niveau de preuve ne remplace pas une sauvegarde fraîche déclenchée depuis le bundle sur les données réelles, suivie d’une restauration et d’une relecture en jeu.
2. **Patch Ollama verrouillé — implémenté et contrôlé.** `third-party.lock.toml` déclare le chemin et le SHA-256 du patch RealmBox ; `cargo xtask release check` refuse une déclaration absente, un autre chemin, un fichier manquant ou une dérive du contenu. Aucun nouvel upstream de cette revue n’entre dans le build. Toute future dépendance devra relier URL, SHA complet, licence, source, patchset, checksum ou digest d’image et preuve de build reproductible.
3. Ajouter au modèle de menace les conversations locales, mémoires de compagnons, exports de préréglages et sauvegardes manuelles. Vérifier qu’aucun de ces artefacts ne quitte la machine ni ne rejoint un diagnostic partageable.

### P1 — rendre l’équipe simple et fiable

1. Livrer des préréglages de groupe 5 joueurs : composition par rôles, noms préférés, formation et stratégies essentielles.
2. Garantir « ne jamais retirer un humain », suspendre les mutations en combat et afficher la raison d’un échec.
3. Dans l’addon, rendre la portée groupe/cible toujours visible, employer des listes fermées, montrer l’action construite et confirmer les opérations dangereuses.
4. Ajouter des profils solo réversibles en FR/EN avec aperçu des règles, sans exposer les fichiers serveur dans le flux normal.

### P2 — fiabiliser dialogue et compagnon persistant

1. Généraliser la file locale bornée : demandes humaines prioritaires, emplacement réservé, budgets par canal, délai, backoff et coupe-circuit.
2. Introduire un compagnon principal persistant par personnage avec rôle, formation, rappel/masquage, préférence d’auto-rappel et mémoire bornée effaçable.
3. Convertir les formulations naturelles en intentions énumérées. Revalider maître, cible, groupe, combat, position et disponibilité juste avant l’action ; ne jamais exécuter du texte libre produit par le modèle.
4. Tester le parcours français et anglais, seul et en groupe, avec Ollama indisponible, lent, bavard ou incohérent.

### P3 — expérimenter sans élargir le risque

1. Prototyper un guide factuel en lecture seule sur des données locales autorisées, avec provenance de réponse, incertitude explicite et repli silencieux.
2. Mesurer un roster 10/25 joueurs et une population répartie par niveaux avant tout format 40 joueurs.
3. Tester AutoBalance uniquement dans une liste d’instances, avec plafonds, exceptions de boss et bouton de retour au comportement standard.
4. Évaluer l’autonomie LLM sur un petit nombre de bots dans un environnement jetable, sans accès aux données joueurs réelles.

### Hors feuille de route tant que P0–P2 ne sont pas prouvés

- changement de cœur vers NPCBots ;
- repack ou distribution de données de jeu ;
- hardcore, progression par extension, arènes, Wintergrasp ou portail web ;
- fournisseur IA hébergé.

Le critère de réussite n’est pas le nombre de fonctions récupérées. C’est la capacité à offrir une équipe plus vivante et plus simple sans affaiblir la propriété locale des données, la récupération du royaume, la provenance des builds ou la compréhension du joueur.
