# État factuel

Mis à jour le 3 septembre 2026 sur macOS 26.6.2 arm64.

Décision actuelle : **GO automatisé pour les sources RealmBox 0.3.2 : concentration progressive des bots autonomes, comportements de l’addon, dialogues bot-à-bot bornés et site Astro FR/EN. Le bundle macOS 0.3.1 est installé et ouvert, mais les comportements 0.3.1–0.3.2 n’ont pas encore été rejoués dans le vrai monde ; la dernière preuve de gameplay reste en 0.2.4. Le brouillon GitHub reste en v0.2.0. NO-GO pour publier une release publique prête pour Windows, signée ou juridiquement validée.**

## Évolution bots 0.3.1–0.3.2

| Fonction | Niveau de preuve | Résultat |
|---|---|---|
| Population proche | test Rust de configuration | bots inactifs sans joueur réel ; priorité au rayon de 300 mètres et à la zone active ; poids upstream 15 sur la tranche de niveau des joueurs, recalculé chaque minute ; téléportation de niveau active et téléportation aléatoire réévaluée entre 2 et 5 minutes |
| Présence progressive | test C++ de politique + tests Rust d’intégration | le module RealmBox vise au plus 60 % des bots autonomes en ligne près des joueurs réels, sans déplacer les bots groupés, en combat, en instance ou visibles par un autre joueur ; parcours réel à effectuer |
| Comportement de groupe | 6 tests Fengari sur le vrai Lua | modes escorte, garde et autonomie bornés ; dissolution qui restaure l’autonomie avant `leave` ; interface réelle OpenWoW à requalifier |
| Dialogues bot-à-bot | tests de patch, tests Rust et 24 tests UI | profils silencieux, équilibré et vivant ; exactement un rebond au plus, groupes pris en charge, files et workers bornés, cooldowns et plafonds par portée/globaux ; changement à chaud par `ollama reload` |
| Réglages à chaud | tests Rust | la configuration AzerothCore est rechargée avant Playerbots/Ollama ; la population peut être changée après redémarrage du launcher tant que le worldserver continue de tourner |
| Bundle macOS local | build + installation + inspection fraîche | RealmBox 0.3.1 arm64 installé dans `/Users/benjamin/Applications/RealmBox.app`, signature ad hoc stricte valide, exécutable SHA-256 `ca0fbea2…84fadab`, application ouverte ; ancien bundle 0.3.0 conservé en sauvegarde |
| Addon embarqué | comparaison SHA-256 | source et ressource du bundle 0.3.1 identiques (`15322fe9…9607ba`) ; le client géré contient encore 0.2.0-dev et sera remplacé atomiquement au prochain démarrage du monde |
| Vérification globale 0.3.2 | automatisée | typecheck, lint, 24 tests UI, 12 tests de scripts, builds Vite et Astro (3 pages), Clippy strict et 51 tests Rust réussis ; contrôle de cohérence de release relancé après la mise à jour du présent statut |

## Durcissement automatisé 0.3.0

| Fonction | Niveau de preuve | Résultat |
|---|---|---|
| Restauration automatique | tests Rust avec runner factice | runtime et dump SQL vérifiés avant échange ; sauvegarde de sécurité non écrasable ; import cible ; validation des quatre bases ; un import volontairement défaillant restaure automatiquement runtime et base initiaux ; aucun `--volumes` |
| Supervision des processus | tests Unix réels + compilation MSVC isolée | seuls les enfants conservés par handle sont arrêtables ; groupe de processus Unix fermé ; Job Object Windows configuré avec `KILL_ON_JOB_CLOSE` ; parcours Windows réel non exécuté |
| Téléchargements joueur | tests Rust + compilation | client HTTP Rust avec proxy système, reprise HTTP, trois tentatives, octets transférés et publication atomique après SHA-256 ; `curl` reste seulement le fallback des runners de test/développement |
| Profils et limites | tests UI/Rust | profils Aventure tranquille, Monde vivant, Monde dense et Personnalisé ; population souhaitée conservée séparément de la valeur plafonnée réellement appliquée |
| Dialogues | tests UI/Rust | espace disponible affiché et marge disque imposée ; trois niveaux de bavardage bornés, modifiables uniquement monde arrêté ; commandes de jeu toujours hors LLM |
| Accessibilité | axe + tests UI | 23 tests React, dont flux principal et modale contrôlés par axe hors contraste non calculable dans jsdom ; VoiceOver et Narrator restent manuels |
| Industrialisation GitHub | lecture API après mutation | jalons 0.3/0.4/1.0, issues #11 à #20 et ruleset `main` actif ; checks Validation, macOS arm64 et Windows x64 obligatoires, résolution des discussions, bypass administrateur |
| Vérification locale | automatisée | `pnpm verify` réussi après mise à jour du statut : typecheck, lint, 23 tests UI, 7 tests de scripts, build Vite, Clippy strict, 49 tests Rust et cohérence de release 0.3.0 |

## Refonte desktop et correctifs 0.2.2–0.2.4

| Fonction | Niveau de preuve | Résultat |
|---|---|---|
| Launcher fixe | configuration + test Node + build Tauri | fenêtre 1024 × 640, min/max identiques, non redimensionnable, non maximisable et centrée ; bundle `.app` accepté par le schéma Tauri 2 |
| Composition joueur | 20 tests UI + QA visuelle simulée | une illustration ImageGen plein cadre, une marque, un état et une action ; réglages, langue, compagnons, dialogues et diagnostic dans un panneau contextuel ; focus piégé dans la modale puis restauré ; états premier lancement, vérification, installation, prêt et erreur contrôlés à 1024 × 640 |
| Progression et erreurs | tests UI + QA visuelle simulée | aucune jauge à 0 ou 100 %, progression visible uniquement pendant une opération active ; détails techniques absents de l’accueil et accessibles dans Diagnostic |
| Protocole et diagnostic partageable | tests UI/Rust | codes d’erreur et récupérations sérialisés par Tauri ; le front ne classe plus les phrases ; secrets et chemins utilisateur expurgés, chemin local exclu de la copie partageable |
| Activation des dialogues après 0.2.0 | tests UI/Rust + parcours réel | l'ouverture ne démarre plus automatiquement le monde ; le blocage monde actif propose un arrêt explicite ; runtime serveur préparé en staging après backup SQL vérifié, ancien serveur conservé hors runtime et projet Compose `realmbox-v3` inchangé ; extraction corrigée depuis `/azerothcore/env/ref/etc/modules/mod_ollama_chat.conf.dist` |
| Dossier client modifiable | 2 tests UI + 3 tests Rust | changement interdit pendant la partie ; nouveau dossier 3.3.5a revalidé ; overlay OpenWoW remplacé atomiquement ou chemin `Wow.exe` mis à jour ; Compose et base joueurs inchangés ; parcours Windows réel non rejoué |
| Addon Compagnons 0.2.0-dev | 5 tests exécutant le Lua + analyse XML | réduction dans une icône de minimap, visibilité et positions persistées, FR/EN, composition du groupe, prérequis d’action et commandes bornées couverts avec API WoW simulée ; rendu et commandes `co ±boost` non encore rejoués dans OpenWoW |
| Bundle macOS local | build + installation + inspection locale | RealmBox 0.2.4 arm64 installé dans `/Users/benjamin/Applications/RealmBox.app`, signé ad hoc ; `codesign --deep --strict` valide ; exécutable SHA-256 `e136e64c…d2c308` ; les quatre références serveur immuables, le prompt bilingue minimal et l’addon de reconnexion sont intégrés |
| Vérification locale | automatisée | `pnpm verify` réussi : typecheck, lint, 20 tests UI, 7 tests de scripts dont 5 exécutant le Lua de l’addon, build Vite, Clippy strict, 43 tests Rust du workspace et contrôle de cohérence de release |

## Jalon 0.2.0

| Fonction | Niveau de preuve | Résultat |
|---|---|---|
| Interface FR/EN | test UI + contrôle visuel local | 13 tests UI réussis ; nouvelle navigation Dialogues contrôlée en aperçu FR, parcours FR/EN couvert par tests |
| Identité visuelle launcher/site | tests UI + build + QA visuelle simulée | panoramas ImageGen distincts optimisés en WebP, icône SVG originale, anciens décors sans provenance ou procéduraux retirés ; launcher `ready` contrôlé à 1140×900 et 780×650, site FR à 1440×1000 et 390×844, bascule EN relue dans l’arbre accessible |
| Responsive du site | aperçu navigateur statique | captures FR/EN à 1440×900, 1280×800, 768×1024, 390×844 et 360×800 ; contrôles supplémentaires 1920×1080 et 1024×768 ; aucun débordement horizontal mesuré |
| Icône native | génération + build macOS | source SVG et variante monochrome, PNG 1024, ICNS et ICO générés ; `CFBundleIconFile=icon.icns`, hash identique à la source générée, bundle ad hoc valide |
| Parcours joueur | test UI + build | vues Mon monde, Compagnons, Dialogues et Diagnostic ; action Jouer dominante et arrêt secondaire en jeu ; build Vite réussi |
| Erreurs récupérables | test UI | cause courte et action affichées dans Mon monde ; détail technique présent uniquement dans Diagnostic |
| Diagnostic | tests UI/Rust | composant, chemin des logs, avertissements/erreurs filtrés, masquage des lignes sensibles et copie |
| Population à chaud | test Rust + preuve Docker isolée | migration Compose idempotente, limite mémoire, commandes `playerbot rndbot reload` et `playerbot rndbot update` réellement consommées par un conteneur éphémère sans redémarrage |
| Protection des personnages lors des mises à jour | tests Rust | réinstallation par-dessus un royaume refusée, schéma inconnu traité en erreur, suppression de volumes interdite, dump des quatre bases vérifié et signé avant `db-import`, sauvegarde privée hors runtime et jamais écrasée |
| Population à chaud en jeu | non prouvé | le runtime réel actif utilise encore l’ancien Compose ; aucun redémarrage de la partie en cours n’a été imposé |
| Activation des dialogues locaux | tests UI/Rust + parcours réel | RealmBox a choisi `llama3.2:3b`, téléchargé Ollama 0.33.2 et 1,9 Go de modèle sur l'hôte, publié le runtime atomiquement et obtenu la réponse locale attendue ; le module du vrai worldserver est chargé et atteint `host.docker.internal:11435` |
| Site GitHub Pages | contrôle visuel + déploiement | page complète FR/EN contrôlée en desktop et mobile, workflow vert, publication HTTPS relue sur `bnjdpn.github.io/RealmBox/` |
| Images serveur | build CI + lecture anonyme | quatre manifestes GHCR immuables authserver, worldserver, db-import et tools, chacun linux-amd64 + linux-arm64 ; pull sans compte réussi dans le run 33687674728 |
| Bundle macOS 0.2.0 | build CI + inspection fraîche | DMG du run 33692911683 retéléchargé, somme interne valide, application arm64 et signature ad hoc vérifiée avec `codesign --deep --strict` ; non notarié |
| Bundle macOS local avec Dialogues | build + installation + inspection locale | `.app` 0.2.0 arm64 installé dans `/Users/benjamin/Applications/RealmBox.app` avec les quatre digests serveur, signature ad hoc stricte valide, exécutable SHA-256 `15314d02…bbf2a` ; non lancé après réinstallation |
| Bundle Windows 0.2.0 | build CI + inspection de format | installateur NSIS produit par Windows 2025 dans le run 33692911683 et retéléchargé ; parcours réel Windows 11 non exécuté |
| Pré-release GitHub | lecture fraîche | brouillon privé `v0.2.0`, DMG + EXE + `SHA256SUMS.txt` présents ; SHA-256 `40d395…b44ae` et `c2275f…ef59c` revérifiés après retéléchargement |
| Vérification locale | automatisée | `pnpm verify` : typecheck, lint, 13 tests UI, 1 test de packaging, build Vite, clippy strict et 50 tests Rust du workspace |

## Preuve réelle sur ce Mac

| Fonction | État | Preuve |
|---|---|---|
| Installation RealmBox | réussie | OpenWoW, AzerothCore, Playerbots, MySQL et les données serveur extraites sont présents dans le runtime géré |
| Données 3.3.5a | réussie | copie build 12340 reconnue ; `maps` 5 744 fichiers, `vmaps` 12 494, `mmaps` 3 748 ; aucune donnée propriétaire ajoutée au dépôt |
| Serveur local | en cours d’exécution | `database`, `authserver` et `worldserver` démarrés ; ports 3724 et 8085 liés à `127.0.0.1` |
| Client OpenWoW | parcours réel réussi | OpenWoW 0.1.2 arm64 charge les données, se connecte à `127.0.0.1:3724`, sélectionne le royaume `RealmBox` et entre dans le monde |
| Compte local | réussi | authentification SRP6 avec `REALMBOX / REALMBOX` ; sel et vérificateur contrôlés indépendamment |
| Personnage et quête | réussi | guerrier humain `Realmbox` créé, entrée à Northshire et quête de départ obtenue |
| Bots autonomes | réussi | configuration à 50 ; la base a confirmé 50 bots connectés avec le joueur, et un bot autonome a été observé dans la zone |
| Équipe de compagnons | réussi | le bouton `Former mon équipe` de l’addon a invoqué à côté du joueur `Kayarid` paladin, `Jillo` prêtre, `Manuela` mage et `Garea` chasseur ; les cinq membres sont confirmés dans le groupe local |
| Mémoire | stable avec 50 bots | Mac : 36 Gio physiques ; Docker Desktop : 15,8 Gio alloués ; `worldserver` observé autour de 5,2 Gio après démarrage, sans nouvel OOM |
| Arrêt manuel et supervision | réussi | le bouton `ARRÊTER` coupe le monde proprement ; le nouveau bundle est revenu automatiquement à l’état prêt quand un processus OpenWoW de lancement s’est terminé |
| Dialogues locaux | parcours réel bilingue réussi | modèle `llama3.2:3b` présent hors conteneur dans le runtime hôte ; module worldserver rechargé à chaud avec réponses joueur à 100 %, un bot par message, zéro cooldown et bavardages automatiques coupés ; une question anglaise a reçu une réponse anglaise de Killat, puis une question française différente a reçu une réponse française de Killat ; le prompt complet observé ne contenait que le message courant |

La preuve visuelle reste locale et n’est pas ajoutée au dépôt, car elle contient des ressources du jeu.

## Version installée

| Changement | Preuve |
|---|---|
| Population réglable | sélecteur 5, 25, 50, 100 ou 150 ; valeur limitée selon la mémoire Docker ; tests Rust sur les paliers et le mode désactivé |
| OpenWoW local sans modifier la copie joueur | overlay `Data` sur macOS/Linux avec `realmlist.wtf` local ; test vérifiant que le fichier source reste intact |
| Réparation du compte local | l’installation met à jour le sel et le vérificateur à chaque passage ; test SQL idempotent |
| Addon Compagnons | le bundle 0.3.1 embarque l'addon 0.3.1-dev ; copie atomique dans le client au prochain démarrage géré, avant l'ouverture de WoW ; gameplay réel encore prouvé avec l'ancienne version |
| Limite Playerbots | guildes aléatoires désactivées pour réduire la mémoire ; capacité recalculée à chaque démarrage |
| Supervision du client | le processus enfant est réclamé par un thread d’attente ; test de régression réel Unix contre l’état zombie |
| Vérification locale du bundle installé | typecheck, lint, 24 tests UI, 10 tests de scripts, build Vite, Clippy strict et 50 tests Rust ; bundle 0.3.1 arm64 inspecté après copie |

Le bundle 0.3.1 est installé et ouvert depuis `/Users/benjamin/Applications/RealmBox.app`, signé localement ad hoc, et son exécutable a le SHA-256 `ca0fbea27a43f2d8c75aac0ca40108f39775039a126393bfc2cafddd184fadab`. Le bundle précédent reste récupérable dans `/Users/benjamin/Applications/RealmBox-0.3.0-backup-20260903-134404.app`. Les volumes `realmbox-v3_realmbox-database` et `realmbox-v3_realmbox-server-data` ont été relus après l'installation. Le modèle n'est pas dans Docker : Ollama et ses modèles sont publiés sous `~/Library/Application Support/org.realmbox.desktop/runtime-v3/ai`.

## Défauts et travaux ouverts

- OpenWoW a affiché une alerte macOS de restauration après une fermeture inattendue. Le lanceur ne doit pas laisser cette alerte produire une instance non supervisée ; la récupération automatique reste à durcir.
- La configuration 0.3.1 favorise désormais fortement la tranche de niveau du joueur, sa zone et son voisinage, mais ce résultat n'a pas encore été mesuré dans le vrai monde et ne constitue pas une garantie de position pour chaque bot.
- Les modes escorte, garde et autonomie ainsi que la dissolution qui rend les bots autonomes passent le harnais Lua simulé, mais doivent encore être observés dans OpenWoW.
- Les échanges bot-à-bot et leur rechargement à chaud passent les tests UI/Rust ; leur cadence et l'absence de flood doivent encore être qualifiées avec le vrai worldserver et le modèle local.
- La nouvelle ergonomie 0.2.0-dev de l’addon Compagnons passe son harnais Lua simulé, mais l’icône de minimap, la persistance, le bilingue et les commandes `co +boost` / `co -boost` doivent encore être observés dans OpenWoW.
- Le contrôle à chaud est implémenté dans 0.2.0 et prouvé avec fakes plus un conteneur Docker isolé. Il reste à le rejouer sur le vrai worldserver après le prochain démarrage géré, sans interrompre la session actuellement ouverte.
- Le contrat de mise à jour protège désormais le volume et impose une sauvegarde avant migration. Trois dumps réels présents ont une empreinte SHA-256 valide, dont ceux créés pendant le parcours 0.2.2 ; la restauration complète passe avec fakes, y compris le retour après import défaillant, mais n’a pas encore été exécutée puis vérifiée en jeu sur les données réelles.
- L'activation Ollama après une installation antérieure est prouvée sur ce Mac avec staging, sauvegarde, rollback du runtime, téléchargement et inférence. Le correctif 0.2.4 a été rechargé par la commande upstream `ollama reload` et les réponses FR puis EN ont été observées dans le canal de groupe. La qualité factuelle reste celle d'un modèle local 3B et n'est pas garantie.
- Le bundle 0.3.1 contient l'addon 0.3.1-dev vérifié, mais le client géré conserve encore l'addon 0.2.0-dev tant qu'un nouveau démarrage du monde n'a pas déclenché sa copie atomique. Son interface et ses nouveaux comportements doivent ensuite être confirmés dans OpenWoW.
- Les images serveur RealmBox sont publiées et épinglées par digest. Le prochain parcours réel doit confirmer qu’une installation neuve les télécharge sans recompiler AzerothCore et Playerbots.
- Windows x64 compile en CI mais n’a pas de parcours réel Windows 11. `Wow.exe` doit devenir le choix recommandé quand une copie compatible est présente ; OpenWoW doit rester optionnel.
- Linux et Mac Intel ne sont pas pris en charge par le produit actuel.
- La signature de distribution et la notarisation macOS restent bloquées faute de certificats.
- Le DMG macOS 0.2.0 du brouillon est construit et vérifié, mais reste signé uniquement en ad hoc et non notarié.
- Les exécutables 0.2.0 sont attachés à une pré-release GitHub en brouillon. Ils ne sont pas publiés au public tant que l’audit des notices, la signature de distribution et le parcours Windows 11 ne sont pas terminés.

## Suite

La feuille de route produit, y compris la refonte visuelle, les profils matériels, les logs d’installation, les bots à chaud et le LLM local, est suivie dans [ROADMAP.md](ROADMAP.md).
