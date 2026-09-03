# Changelog

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
- Remplace le prompt de conversation par un gabarit minimal et déterministe : seul le dernier message joueur décide du français ou de l’anglais, sans historique susceptible d’imposer la langue précédente.
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
