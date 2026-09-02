# État factuel

Mis à jour le 2 septembre 2026 sur macOS 26.6.2 arm64.

Décision actuelle : **GO pour le parcours local macOS avec OpenWoW, AzerothCore et Playerbots. GO automatisé pour RealmBox 0.2.0 et son site FR/EN. NO-GO pour déclarer le réglage Playerbots à chaud prouvé dans un vrai worldserver, pour une release publique prête pour Windows, pour les dialogues LLM en jeu et pour l’affirmation que toutes les fonctions OpenWoW sont équivalentes au client original.**

## Jalon 0.2.0

| Fonction | Niveau de preuve | Résultat |
|---|---|---|
| Interface FR/EN | test UI + contrôle visuel local | 10 tests UI réussis ; vues FR et EN contrôlées en 1200 × 960 |
| Parcours joueur | test UI + build | vues Mon monde, Compagnons et Diagnostic ; textes décoratifs retirés ; build Vite réussi |
| Erreurs récupérables | test UI | cause courte et action affichées dans Mon monde ; détail technique présent uniquement dans Diagnostic |
| Diagnostic | tests UI/Rust | composant, chemin des logs, avertissements/erreurs filtrés, masquage des lignes sensibles et copie |
| Population à chaud | test Rust + preuve Docker isolée | migration Compose idempotente, limite mémoire, commandes `playerbot rndbot reload` et `playerbot rndbot update` réellement consommées par un conteneur éphémère sans redémarrage |
| Population à chaud en jeu | non prouvé | le runtime réel actif utilise encore l’ancien Compose ; aucun redémarrage de la partie en cours n’a été imposé |
| Site GitHub Pages | contrôle visuel local | page complète FR/EN, viewport desktop et mobile ; publication distante à vérifier après le push |
| Bundle macOS 0.2.0 | build + inspection locale | DMG généré, somme interne valide, application arm64 et signature ad hoc vérifiée avec `codesign --deep --strict` ; non notarié |
| Vérification locale | automatisée | `pnpm verify` : typecheck, lint, 10 tests UI, build Vite, clippy strict et 42 tests Rust du workspace |

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

La preuve visuelle reste locale et n’est pas ajoutée au dépôt, car elle contient des ressources du jeu.

## Version installée

| Changement | Preuve |
|---|---|
| Population réglable | sélecteur 5, 25, 50, 100 ou 150 ; valeur limitée selon la mémoire Docker ; tests Rust sur les paliers et le mode désactivé |
| OpenWoW local sans modifier la copie joueur | overlay `Data` sur macOS/Linux avec `realmlist.wtf` local ; test vérifiant que le fichier source reste intact |
| Réparation du compte local | l’installation met à jour le sel et le vérificateur à chaque passage ; test SQL idempotent |
| Addon Compagnons | bouton d’équipe équilibrée, commandes suivre/attaquer/attendre/regrouper/libérer ; parcours réel validé dans la session en cours |
| Limite Playerbots | guildes aléatoires désactivées pour réduire la mémoire ; capacité recalculée à chaque démarrage |
| Supervision du client | le processus enfant est réclamé par un thread d’attente ; test de régression réel Unix contre l’état zombie |
| Vérification locale du bundle installé | preuve antérieure : typecheck, lint, 7 tests UI, build Vite, clippy strict, 24 tests desktop Rust et tous les tests/doc-tests du workspace |

Le bundle courant est installé dans `/Users/benjamin/Applications/RealmBox.app`, signé localement ad hoc, relancé avec son WebView et son monde local, et son exécutable a le SHA-256 `f0a329169c03abbb101f941f7104b37109f70190014c3abc8429120917230000`.

## Défauts et travaux ouverts

- OpenWoW a affiché une alerte macOS de restauration après une fermeture inattendue. Le lanceur ne doit pas laisser cette alerte produire une instance non supervisée ; la récupération automatique reste à durcir.
- Les 50 bots autonomes sont répartis dans Azeroth. Seuls les quatre bots d’équipe sont garantis près du joueur.
- Le contrôle à chaud est implémenté dans 0.2.0 et prouvé avec fakes plus un conteneur Docker isolé. Il reste à le rejouer sur le vrai worldserver après le prochain démarrage géré, sans interrompre la session actuellement ouverte.
- Ollama et `mod-ollama-chat` sont intégrés et épinglés, mais aucun modèle n’a encore été téléchargé et aucune conversation en jeu n’est prouvée.
- Sans images serveur RealmBox publiées, le bundle de développement compile encore AzerothCore et Playerbots lors de la première installation. Le parcours joueur cible doit uniquement télécharger des images épinglées.
- Windows x64 compile en CI mais n’a pas de parcours réel Windows 11. `Wow.exe` doit devenir le choix recommandé quand une copie compatible est présente ; OpenWoW doit rester optionnel.
- Linux et Mac Intel ne sont pas pris en charge par le produit actuel.
- La signature de distribution et la notarisation macOS restent bloquées faute de certificats.
- Le DMG macOS 0.2.0 est construit et vérifié localement, mais reste signé uniquement en ad hoc et non notarié.
- Le site Pages et les exécutables 0.2.0 ne sont pas déclarés publiés avant lecture fraîche des workflows et des artefacts GitHub.

## Suite

La feuille de route produit, y compris la refonte visuelle, les profils matériels, les logs d’installation, les bots à chaud et le LLM local, est suivie dans [ROADMAP.md](ROADMAP.md).
