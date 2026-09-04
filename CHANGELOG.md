# Changelog

## 0.5.0 — 2026-09-04 (préversion en préparation)

- Remplace les options de première installation cachées par un assistant FR/EN en trois étapes, avec aide au client WoW, cartes de population, présence indépendante, dialogue facultatif et récapitulatif avant téléchargement.
- Vérifie en lecture seule Docker/Compose, la plateforme, la destination et l’espace disque ; garde Installer désactivé si un contrôle manque ou échoue, invalide les vérifications obsolètes et conserve les choix après erreur. Un lien d’installation cassé n’est jamais considéré comme une installation neuve.
- Ajoute à l’accueil les raccourcis du royaume, une population explicitement configurée plutôt qu’un faux compteur en ligne, l’aide de connexion et le suivi de l’étape d’installation réelle. Les détails techniques restent dans Diagnostic.

- Ajoute trois profils solo FR/EN, inspectables et réversibles, limités à onze réglages AzerothCore. Les instantanés et journaux sont synchronisés, relus, publiés sans écrasement et repris avant le lancement ; aucune donnée de personnage n’est réécrite.
- Ajoute un guide local explicite pour les noms de quêtes et d’objets : huit résultats maximum, source et incertitude visibles, aucune IA, aucun contexte personnage et deux requêtes MySQL fixes exécutées dans une transaction en lecture seule, avec délai SQL et limites distinctes des processus Docker.
- Étend l’addon avec trois préréglages d’équipe à cinq, un compagnon principal observé, une portée groupe/cible et l’aperçu de la commande. Il ne retire aucun membre et ne promet pas le rappel nominatif tant que Playerbots ne fournit pas la validation atomique requise.
- Ajoute au dialogue local un coupe-circuit partagé : pause après trois échecs, une seule sonde de reprise et attente exponentielle plafonnée à 60 secondes, sans nouvelle tentative ni thread non borné.
- Empêche deux instances RealmBox de piloter simultanément le même runtime et tente de refermer une base temporairement démarrée par le guide, y compris après un échec partiel ou un timeout ; un Docker inaccessible laisse l’arrêt du conteneur à vérifier.
- Conserve le socle et les révisions immuables existants : aucun repack, dump, client, donnée extraite, fournisseur IA distant ou nouvelle dépendance des huit projets étudiés n’est importé.
- Cette section décrit l’arbre source. Aucun bundle, image serveur intégrant le nouveau patch C++, release publique ou parcours OpenWoW réel n’est revendiqué à ce stade ; la version Cargo passe à 0.5.0 pour déclencher la sauvegarde obligatoire avant migration.

## 0.4.0 — 2026-09-03

- Ajoute dans Réglages une vue **Protection** bilingue qui crée à la demande un nouveau dump cohérent des quatre bases, vérifie son contenu et son SHA-256, le conserve hors du runtime sans écrasement et laisse un monde déjà ouvert en fonctionnement.
- Verrouille dans le manifeste de provenance le correctif RealmBox appliqué au module Ollama et documente les idées retenues ou écartées après revue des principaux projets solo, Playerbots, NPCBots et assistants locaux.
- Sépare les cinq populations proposées — 5, 25, 50, 100 et 150 — de leur présence autour du joueur avec trois choix explicites : **Dispersés**, **Naturelle** (recommandé) et **Toujours proches**. Les installations neuves choisissent Naturelle ; une installation antérieure à 0.4.0 sans préférence enregistrée conserve Toujours proches pour éviter un changement silencieux.
- Rend chaque profil de présence cohérent de bout en bout dans Playerbots et `mod-realmbox-presence`, puis rend les bots déplacés au cycle de voyage natif avec `ScheduleTeleport` après un délai borné. Le mode Dispersés peut raccourcir ce délai pour les bots placés et encore suivis par la même instance serveur, dès qu’ils sont sûrs et hors de vue, sans téléportation immédiate ; il ne devine pas l’origine des événements hérités après un redémarrage.
- Présente **Escorte**, **Garde** et **Libres** comme trois choix séparés dans l’addon. La préférence enregistrée n’est réappliquée qu’après stabilisation du groupe ; libérer l’équipe restaure d’abord les stratégies autonomes avant `leave`.
- Borne à 30 secondes la réapplication différée d’un comportement d’équipe, conserve le modèle local lors d’une réactivation hors ligne et resynchronise toutes les préférences après restauration d’un runtime.
- Remplace les libellés techniques de bavardage par **Direct**, **Immersif** et **Vivant**. Direct coupe l’ambiance ; Immersif utilise des occasions modérées et un intervalle de 90–180 s ; Vivant augmente ces occasions avec un intervalle de 30–90 s, toujours sous plafonds par portée et global.
- Place les demandes humaines éligibles, configurées à 100 %, avant les tâches ambiantes, réserve un emplacement de file au trafic humain et sépare les budgets des groupes et raids. Un changement de mode invalide l’ambiance en attente ou en vol sans supprimer les demandes humaines. Une file humaine pleine, un échec Ollama ou une destination disparue peuvent toujours empêcher la réponse.
- Désactive l’historique, la mémoire, les relations, le RAG, le sentiment et les emotes générées dans les profils RealmBox. Le prompt direct demande une réponse dans la langue du dernier message ; les prompts ambiants sont français pour une copie `frFR` et anglais pour les autres locales prises en charge.
- Ajoute une couverture automatisée des profils, migrations de préférences, commandes bornées et invariants structurels du patch. La cadence de présence, le rendu de l’addon et les nouveaux comportements de dialogue restent à qualifier dans un parcours OpenWoW réel.

## 0.3.4 — 2026-09-03

- Détecte la disparition des volumes Docker gérés et annonce leur reconstruction au prochain lancement du monde.
- Recrée les ressources Docker supprimées, restaure automatiquement la dernière sauvegarde SQL complète et vérifiée, puis reprend les migrations avant d’ouvrir le client ; sans sauvegarde valide, RealmBox échoue fermé plutôt que de créer un royaume vide.
- Rend le binaire Docker Desktop et ses helpers d’identifiants accessibles aux processus lancés depuis l’application macOS, afin que les images épinglées puissent être retéléchargées après une purge.
- Remplace une image serveur locale disparue par l’ensemble d’images de release immuables embarqué dans le launcher, sans changer le projet Compose `realmbox-v3` ni demander la suppression d’un volume.
- Embarque et monte la configuration MMaps du commit serveur épinglé, y compris lors de la reprise d’un ancien Compose, afin que les données de navigation soient réellement régénérées après la purge.
- Nettoie uniquement l’intermédiaire `Buildings` et les sorties VMaps/MMaps générées et incomplètes avant leur régénération, afin qu’une reconstruction interrompue puisse reprendre sans toucher au volume de la base joueurs.
- Classe les échecs de reconstruction comme erreurs serveur et ignore les noms de fichiers contenant seulement une sous-chaîne telle que `Warningtree` dans le diagnostic.

## 0.3.3 — 2026-09-03

- Réapplique à chaque démarrage une présence dense autour du joueur : un bot autonome de même faction par seconde, placé à 30–90 mètres, jusqu'à la cible existante de 60 %.
- Aligne le rayon de présence sur le garde-fou de visibilité à 150 mètres afin que les bots protégés du déplacement soient également comptés comme proches.
- Réduit à dix secondes le délai avant qu'un bot autonome puisse suivre un nouveau changement de zone et aligne le bavardage local du profil Vivant sur le rayon de présence de 150 mètres.

## 0.3.2 — 2026-09-03

- Ajoute un module serveur RealmBox qui rassemble progressivement jusqu’à 60 % des bots autonomes en ligne près des joueurs réels, sans déplacer les bots groupés, en combat, en instance ou visibles par un autre joueur.
- Empêche le planificateur Playerbots d’annuler le placement, conserve une activité autonome en arrière-plan et accorde cinq minutes de grâce après la libération d’un bot pour qu’il reprenne sa vie propre.
- Corrige le mode bavardage : une origine peut recevoir exactement un rebond bot-à-bot, les groupes sont pris en charge, les files et workers LLM sont bornés, et le plafond d’initiateurs par groupe/zone est désormais réellement appliqué.
- Recharge d’abord la configuration AzerothCore avant les commandes Playerbots/Ollama et permet les changements de population après le redémarrage du launcher tant que le worldserver tourne.

## 0.3.1 — 2026-09-03

- Concentre la population autonome sur la tranche de niveau du joueur et donne la priorité aux bots proches ou présents dans sa zone, avec réévaluation périodique upstream.
- Ajoute à l’addon les modes escorte, garde et autonomie ; libérer le groupe rétablit désormais l’autonomie avant de faire quitter les bots.
- Permet de régler le bavardage à chaud quand le LLM local est actif et autorise, selon le profil, des échanges bot-à-bot à profondeur, concurrence, cooldown et débits strictement bornés.

## 0.3.0 — 2026-09-03

- Ajout de `cargo xtask release check` et d’un manifeste partagé par le site pour bloquer les versions ou statuts de plateforme incohérents.
- Suppression des anciens crates de démonstration qui ne pilotaient pas l’application Tauri.
- Remplacement de l’interprétation des phrases d’erreur côté React par des codes d’erreur sérialisés et des actions de récupération stables.
- Expurgation des chemins utilisateur dans les diagnostics partageables et ajout d’un piège de focus avec restauration dans la fenêtre de réglages.
- Ajout des builds Tauri macOS arm64 et Windows x64 aux pull requests et regroupement mensuel des mises à jour Dependabot mineures/correctives.
- Ajout de la restauration vérifiée du dernier runtime fonctionnel et de sa sauvegarde SQL, avec sauvegarde de sécurité puis retour automatique à l’état initial si l’import échoue.
- Supervision des seuls processus créés par RealmBox au moyen de groupes Unix et de Job Objects Windows fermés avec l’application.
- Remplacement de `curl` par un client HTTP Rust avec reprise bornée, progression en octets, proxy système et publication seulement après contrôle SHA-256.
- Ajout des profils de population, de la distinction souhaitée/appliquée, du contrôle d’espace disque et de trois niveaux de bavardage local.
- Ajout d’un contrôle axe automatisé, des jalons/issues de durcissement et d’un ruleset `main` exigeant les checks commun, macOS et Windows.

## 0.2.4 — 2026-09-03

- Ajoute à l’addon une icône de minimap déplaçable, la réduction/restauration du panneau, un bouton de fermeture, les commandes `/realmbox` et `/rb`, ainsi que la mémorisation des positions et de la visibilité.
- Rend l’addon bilingue FR/EN, affiche la composition connectée du groupe et les membres hors ligne, puis désactive les ordres impossibles en expliquant leur prérequis.
- Remplace la commande inexistante `cooldowns on` par le vrai contrôle Playerbots `co +boost` / `co -boost`, affiché honnêtement comme une préférence envoyée sans accusé serveur.
- Exécute le Lua de l’addon dans cinq tests Fengari avec une API WoW simulée, en plus de valider la structure XML et les métadonnées 3.3.5a.
- Remplace le prompt de conversation par un gabarit minimal et déterministe qui demande au modèle de suivre la langue du dernier message joueur, sans historique susceptible d’imposer la langue précédente.
- Réinstalle atomiquement l’addon compagnon intégré à chaque démarrage afin que les correctifs atteignent aussi les mondes déjà installés.
- Remplace automatiquement les compagnons hors ligne au lieu de considérer leurs emplacements de groupe comme occupés.

## 0.2.3 — 2026-09-03

- Ajoute dans Réglages le changement du dossier du client après installation, avec nouvelle validation des données 3.3.5a et blocage pendant la partie.
- Reconstruit atomiquement l’overlay de données OpenWoW ou met à jour `Wow.exe`, sans réinstallation du serveur ni modification de la base joueurs.
- Sépare les sauvegardes de `realmlist.wtf` par copie du jeu afin de ne jamais écraser la configuration d’un autre client.
- Rend les réponses aux joueurs déterministes dans les canaux pris en charge, avec une seule réponse par message et sans délai artificiel entre deux questions.
- Demande explicitement au modèle de répondre dans la langue du dernier message et de traiter directement les questions.
- Désactive le bavardage aléatoire et événementiel du preset joueur afin qu'il ne concurrence pas les conversations.

## 0.2.2 — 2026-09-03

- Corrige l’extraction de la configuration `mod-ollama-chat` depuis le chemin réellement présent dans l’image serveur immuable.

## 0.2.1 — 2026-09-03

- Remplacement complet du dashboard desktop par un launcher fixe 1024 × 640 : illustration ImageGen plein cadre, marque unique, état courant et action principale ; langue, compagnons, dialogues et diagnostic passent dans un panneau de réglages contextuel.
- La progression n’apparaît que pendant une opération réellement en cours et les erreurs restent en une phrase d’action, avec les détails techniques réservés au diagnostic.
- L’ouverture du lanceur ne démarre plus automatiquement le monde : le joueur peut préparer les dialogues avant de jouer.
- L’écran Dialogues explique le blocage quand le monde tourne et permet de l’arrêter explicitement.
- Une installation 0.2.0 sans module Ollama passe par un runtime serveur précompilé préparé en staging, après sauvegarde SQL complète vérifiée et sans changer le projet Docker `realmbox-v3`.
- L’ancien serveur est conservé hors du runtime actif pour rollback ; le modèle reste téléchargé uniquement après confirmation.

## 0.2.0 — 2026-09-02

- Nouvelle interface joueur avec panorama raster ImageGen original, cadre acier/laiton, plaques de navigation, bouton Jouer dominant et vues Monde, Compagnons, Dialogues et Diagnostic.
- Interface complète en français et en anglais, choix de langue conservé localement.
- Modification de la population Playerbots à chaud via les commandes console upstream, sans redémarrer le client.
- Erreurs joueur courtes avec cause et récupération ; détails techniques et logs filtrés réservés au diagnostic copiable.
- Site GitHub Pages bilingue avec tutoriel macOS/Windows, limites de preuve et vérification SHA-256.
- Workflow de prerelease macOS arm64 et Windows x64 avec fichier `SHA256SUMS.txt`.
- Nouvelle icône portail R/B générée dans tous les formats Tauri, design system commun et provenance complète des assets.

## 0.1.0-dev

- Remplacement du faux parcours desktop et des images générées par un lanceur code-native inspiré de la structure des lanceurs MMO de l'ère Wrath.
- Ajout du premier installateur macOS arm64 : OpenWoW officiel vérifié, sources serveur et Playerbots épinglées, build Docker et extraction locale des données serveur.
- Ajout de l'état d'installation atomique, du démarrage automatique aux lancements suivants, de l'option Playerbots et de l'arrêt ordonné.
- Ajout d'un compte joueur local idempotent et de tests du calcul SRP6.
- Conservation des crates fake historiques comme banc de tests isolé, hors du flux produit Tauri actuel.
- Nouvelle composition visuelle fidèle aux proportions du launcher 3.3.5a, avec une scène fantasy originale générée pour RealmBox et aucun asset Blizzard.
- Ajout du conseil CanIRun borné, de l'installation optionnelle Ollama 0.33.2 + `mod-ollama-chat`, du modèle local en allowlist et de l'arrêt coordonné de l'IA.
- Ajout de la supervision du client : la fermeture d'OpenWoW déclenche l'arrêt automatique du monde, de la base et du moteur de dialogue.
