# État factuel

Mis à jour le 4 septembre 2026. Les preuves de parcours natif antérieures restent datées séparément ci-dessous.

## Synchronisation de la release — 4 septembre 2026

Correction d’orchestration : construction serveur appelée par la release, images isolées par commit/exécution/tentative, quatre empreintes transmises directement aux installateurs après validation. Reprise manuelle d’un tag existant sans déplacement, et fichiers de provenance joints aux artefacts. Tests de régression de dépendances et exécution du contrôle shell sur références valides/invalides ajoutés. Aucun code produit ni donnée joueur modifié dans ce correctif. La 0.5.0 est publique ; le workflow précédent a été arrêté avant publication d’installateurs. La réussite de la nouvelle chaîne CI et les nouveaux artefacts restent à vérifier séparément.

Validation locale du correctif : `actionlint` réussi pour les deux workflows ; `pnpm verify` réussi avec 52 tests React, 32 tests de scripts (dont les quatre nouveaux), 141 tests desktop Rust et 5 tests xtask. Ce résultat n’est pas une preuve de construction ou publication des images/installateurs distants.

## RealmBox 0.5.0 — préparation de la préversion

La version produit et Cargo est incrémentée à 0.5.0 pour la sauvegarde obligatoire avant migration. La publication visée reste un brouillon de préversion : les nouvelles images serveur, les installateurs CI et le parcours réel doivent être vérifiés séparément. Les preuves ci-dessous restent datées ; elles ne qualifient pas automatiquement cette nouvelle version.

## Installation guidée et accueil — 4 septembre 2026, source non publiée

**GO source et vérifications automatisées ; pas de nouveau bundle installé, distribué ou qualifié en jeu.** Le parcours s’inspire de la page App et de la capture publique WOW Legends, pas d’une exécution de son application réservée aux soutiens. Voir [le contrat UX](docs/SETUP_EXPERIENCE.md).

| Élément | Preuve | Résultat |
|---|---|---|
| Assistant en trois étapes | tests React avec fakes | copie WoW inspectée avant la suite, cartes de bots, présence indépendante, IA locale opt-in, récapitulatif, retours conservés, annulation du sélecteur et retry sans réinstallation automatique |
| Préparation avant téléchargement | Rust/fakes + fichiers temporaires | plateforme, destination, espace disque et deux commandes Docker en lecture seule bornées à 10 s chacune ; un lien cassé, un royaume existant, un espace inconnu ou insuffisant ne sont pas déclarés prêts ; aucun démarrage de service ni accès SQL |
| Limites et concurrence | tests React/Rust | population demandée distincte du plafond prévu ; vérification disque invalidée lors d’un changement de modèle, réponse obsolète ignorée ; destination protégée et modèle arbitraire refusé |
| Accueil et aide | tests React/axe + navigateur simulé 1024×640 | raccourcis bots/dialogue/solo/protection/guide, population configurée (pas de télémétrie inventée), aide de connexion, progression de l’opération réelle et détails techniques dans Diagnostic |
| Langues et liens | tests React + catalogue Rust fermé | FR/EN, sélecteur natif titré dans la langue choisie, liens limités à ChromieCraft FR/EN et Docker Desktop ; navigation OS native non exécutée pendant cette validation |
| Vérification globale | `pnpm verify` réussi | **52 tests React, 28 tests de scripts, 141 tests desktop Rust, 5 tests xtask** ; typecheck, lint, builds Vite/Astro, rustfmt, Clippy strict et cohérence de release réussis |

Le navigateur utilise exclusivement des données de démonstration. L’application installée et les données du royaume n’ont pas été modifiées. Le parcours complet avec sélection native, téléchargement, extraction, démarrage et connexion reste à requalifier sur macOS et Windows. La version Cargo a été incrémentée à 0.5.0 pour préparer la préversion.

## Dernière preuve historique de récupération native

Observation historique du 3 septembre : **GO source, build, installation locale et parcours réel pour la récupération Docker RealmBox 0.3.4 : la purge a été détectée, le dump vérifié restauré, Maps/VMaps/MMaps régénérés, le marqueur retiré, le serveur et OpenWoW lancés, les personnages relus et un personnage existant rejoué à Stormwind. Le bundle corrigé était installé dans `/Users/benjamin/Applications/RealmBox.app`; sa relance depuis ce chemin n’avait pas été forcée afin de ne pas interrompre la partie restaurée encore ouverte. Aucune release publique 0.3.4 n’était revendiquée. L’état historique du brouillon GitHub (v0.2.0) n’est pas une relecture actuelle. NO-GO pour publier une release publique prête pour Windows, signée ou juridiquement validée.**

Le workspace courant porte désormais **RealmBox 0.4.0 plus des changements source non publiés**, mais ce rework bots, solo, guide et dialogues reste **NO-GO pour une qualification réelle ou une release** : les sources et contrôles ciblés sont distincts, tandis que l’installation et le parcours de récupération qualifiés ci-dessous concernent le bundle 0.3.4 construit avant ces changements. Aucun nouveau parcours OpenWoW, build complet du worldserver intégrant le coupe-circuit, image serveur, bundle distribué ou release ne prouve encore ce lot.

## Adaptation de l’écosystème — sources non publiées après 0.4.0

| Fonction | Niveau de preuve | Résultat |
|---|---|---|
| Profils solo | tests Rust fichiers temporaires + tests UI | **Normal**, **Confort** et **Accéléré** pilotent uniquement onze clés ; aperçu exact, conservation des lignes non gérées, snapshot privé vérifié, publication non écrasante, journal reprenable et retour ciblé ; aucune donnée personnage réécrite |
| Coupure et concurrence | tests Rust, dont second processus réel | un journal/snapshot n’est publié qu’après écriture, synchronisation et relecture ; une corruption finale échoue fermée ; une seule instance native pilote le runtime ; le monde reprend d’abord une modification interrompue, même si le catalogue de profils a changé |
| Guide local | tests unitaires/UI + MySQL 8.4.11 isolé réel | recherche FR/EN de 2–64 caractères, huit quêtes ou objets maximum, provenance et incertitude ; SQL fixe, transaction read-only et délai SQL 2 s ; processus hôtes bornés séparément à 10 s (inspection/recherche), 125 s (démarrage) et 15 s (arrêt) ; l’isolat sans réseau/port/volume refuse réellement un `INSERT` ; aucune base joueur consultée |
| Escouades et cible | 20 tests Fengari sur le vrai Lua + XML | trois compositions de cinq, préférences par préréglage, compagnon principal observé, portée groupe/cible, aperçu, confirmation 8 s, expiration 30 s, blocage combat et aucune expulsion ; rappel des mêmes bots non promis |
| Dialogue local résilient | test structurel du patch + compilation/exécution C++17 de la politique | après trois échecs : pause 5 s, sonde unique puis 10/20/40/60 s ; réussite/rechargement réinitialise, réponse d’une génération ancienne ignorée ; ni retry, ni thread, ni sommeil ajouté |
| Sécurité des opérations | tests Rust + revue ciblée | volume joueurs absent : aucune création par sauvegarde/guide ; base démarrée seulement pour une recherche locale avec `--no-build --pull never --no-deps`, puis arrêt tenté aussi après échec partiel ou timeout ; tuer le CLI possédé ne garantit pas l’arrêt du conteneur si Docker est inaccessible ; aucun repack, dump ou nouvelle dépendance étudiée importé |
| Interface FR/EN | tests React/axe + aperçu Playwright 1024×640 | navigation Profils solo et Guide local lisible, états complet/vide/partiel/indisponible et erreur de changement incertaine ; captures simulées uniquement, pas le bundle Tauri |
| Vérification globale | automatisée | `pnpm verify` réussi : typecheck et lint, 38 tests React, 28 tests de scripts, builds Vite et Astro (3 pages), rustfmt, Clippy strict, 135 tests desktop Rust (dont vrais processus pour timeout, sortie volumineuse et descendants), 5 tests `xtask` et cohérence de release ; `pnpm test:guide-sql` réussi séparément sur MySQL isolé, conteneur ensuite absent |

Décision de ce lot : **GO pour la source et les preuves automatisées ciblées ; NO-GO pour distribution et qualification gameplay.** L’application installée, le runtime actif éventuel et les données du joueur n’ont pas été modifiés. La prochaine distribution devra incrémenter la version Cargo avant tout artefact.

## Rework bots et dialogues 0.4.0

| Fonction | Niveau de preuve | Résultat |
|---|---|---|
| Réglages indépendants | tests UI/Rust ciblés | population 5/25/50/100/150 avec plafond de sécurité, présence **Dispersés / Naturelle / Toujours proches**, comportement **Escorte / Garde / Libres** et discussion **Direct / Immersif / Vivant** ne se déduisent plus les uns des autres |
| Protection joueur | tests UI et Rust ciblés avec fakes | une vue FR/EN crée un dump cohérent des quatre bases, contrôle son contenu et son SHA-256, ne réutilise ni n’écrase un point existant et laisse une base déjà active en ligne ; aucune sauvegarde réelle n’a encore été déclenchée par cette vue |
| Présence | tests Rust de génération + 8 tests C++ de politique + compilation du module | trois configurations bornées et retour au scheduler Playerbots avec `ScheduleTeleport` ; Dispersés n’accélère que les échéances des bots placés et encore suivis par l’instance serveur courante, car un événement hérité est indiscernable d’un événement natif après redémarrage ; cadence et rendu visuel non observés dans OpenWoW |
| Comportement de l’équipe | harnais Fengari sur le vrai Lua | trois boutons explicites, préférence réappliquée après stabilisation du groupe et autonomie envoyée avant `leave` ; l’addon ne reçoit pas d’accusé structuré du serveur et le rendu réel reste à vérifier |
| Discussion Direct | configuration + contrôles structurels | ambiance aléatoire, événementielle et bot-à-bot coupée ; demandes humaines éligibles configurées à 100 % et prioritaires, sans garantie en cas de file humaine pleine, panne du modèle ou destination disparue |
| Discussion Immersif | configuration + contrôles structurels | aléatoire 20 %, événements 8 %/1 %, réponses bot en dire 20 % et groupe 50 %, intervalle 90–180 s, plafonds ambiants 2 par portée et 4 globalement |
| Discussion Vivant | configuration + contrôles structurels | aléatoire 35 %, événements 10 %/2 %, réponses bot en dire 35 % et groupe 100 %, intervalle 30–90 s, plafonds ambiants 4 par portée et 6 globalement |
| Changement à chaud | contrôles structurels du patch | l’ambiance en file ou déjà en vol est invalidée par génération, tandis que les demandes humaines sont conservées ; rendu réel non observé |
| Contexte conversationnel | inspection de la configuration générée | historique, mémoire, relations, RAG, sentiment et emotes générées désactivés ; réponse directe guidée par le dernier message joueur |
| Langue ambiante | test Rust de génération | les prompts sans message joueur sont français pour une copie `frFR` et anglais pour les autres locales prises en charge ; rendu réel encore non qualifié |
| Compilations serveur ciblées | builds C++ séparés | worldserver compilé avec Playerbots + Ollama patché ; `mod-realmbox-presence` compilé et lié séparément avec Playerbots ; aucun build unique des trois modules ni démarrage runtime 0.4.0 |
| Vérification globale du worktree | `pnpm verify` | typecheck et lint, 31 tests UI, 15 tests de scripts, builds Vite/Astro, Clippy strict, 71 tests desktop, 5 tests `xtask` et contrôle du patch déclaré réussis ; cela ne prouve ni un bundle ni un parcours OpenWoW 0.4.0 |
| Parcours complet 0.4.0 | non prouvé | aucun nouveau parcours OpenWoW ne qualifie la présence, les boutons, la priorité sous charge, deux groupes simultanés, le bot-à-bot, le rendu de la langue ambiante ou l’absence de flood |

## Récupération Docker 0.3.4

| Fonction | Niveau de preuve | Résultat |
|---|---|---|
| Détection de purge | tests Rust + lecture UI réelle | bootstrap prêt mais explicite lorsque les volumes gérés manquent ou qu’un marqueur de reprise subsiste ; le clic sur Jouer passe en récupération |
| Données joueurs | tests Rust + parcours réel | sélection du dump SQL complet et vérifié le plus récent, marqueur hors Docker, restauration et validation avant `db-import` ; marqueur retiré seulement après disponibilité ; deux personnages existants relus à l’écran de sélection et personnage niveau 80 rejoué à Stormwind |
| Reconstruction runtime | tests Rust + parcours réel | remplacement d’une image locale disparue par l’ensemble immuable embarqué ; projet `realmbox-v3` conservé, aucun `--volumes`, Maps/VMaps/MMaps régénérés puis serveur et client ouverts |
| Reprise d’extraction | tests Rust + interruptions réelles | configuration MMaps épinglée embarquée et montée ; les sorties dérivées `Buildings`, `vmaps` et `mmaps` sont nettoyées juste avant leur producteur, ce qui a permis de reprendre après deux répertoires partiels sans toucher à la base joueurs |
| Docker Desktop macOS | test Rust + pull réel | le `PATH` enfant inclut le dossier des helpers `docker-credential-*`, même depuis un bundle ouvert par Finder ; pull des images et recréation des ressources réussis |
| Vérification globale | `pnpm verify` | typecheck, lint, 27 tests UI, 14 tests de scripts, builds Vite/Astro, Clippy strict, 63 tests Rust et cohérence de release réussis |
| Bundle macOS corrigé | build + signature + parcours réel | bundle arm64 0.3.4 signé ad hoc, `codesign --deep --strict` valide, SHA-256 exécutable `8c037e41…9546f6` ; parcours réel exécuté depuis ce bundle |
| Installation locale | échange atomique + lecture fraîche | `/Users/benjamin/Applications/RealmBox.app` contient le build corrigé 0.3.4 (`8c037e41…9546f6`), signature ad hoc stricte valide ; l’ancien build (`9611e158…e42a2`) reste récupérable dans `/private/tmp/RealmBox-0.3.4-pre-recovery-install-20260903-1722.app`; manifeste, SQLite et neuf dumps SQL inchangés ; pas de relance forcée pendant la partie ouverte |

## Évolution bots 0.3.3

| Fonction | Niveau de preuve | Résultat |
|---|---|---|
| Présence persistante | tests Rust de génération | à chaque démarrage, le lanceur écrit une passe par seconde, un rayon proche de 150 mètres et un placement à 30–90 mètres ; le cooldown bot de dix secondes permet de suivre un changement de zone sans cibler Stormwind |
| Bavardage à proximité | tests Rust de génération | le profil Vivant conserve un bavardage local borné et utilise le même rayon de 150 mètres autour du joueur |
| Bundle macOS 0.3.3 | build + installation + readback serveur | bundle arm64 installé, signature ad hoc stricte valide, SHA-256 exécutable `17bfb70e…ae9f5` ; worldserver prêt avec présence `150 yd`, placement `30..90 yd` et bavardage local `150 yd` ; gameplay laissé à la validation de l’utilisateur |

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
