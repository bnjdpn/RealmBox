# Tests

`pnpm verify` exécute typecheck, lint, Vitest, build Vite, formatage, Clippy et tous les tests Rust.

Le lanceur actuel teste la sélection des données possédées, le déclenchement de l'installation, le rendu d'un second lancement déjà démarré, le refus UI de l'IA sur une machine non recommandée, la validation minimale du dossier `Data`, les pins et ports de la composition Docker, le preset Ollama local et borné, l'allowlist de modèles, le payload matériel CanIRun, le vecteur SRP6 du compte local, la frontière de commandes système et l'arrêt automatique après disparition du client possédé.

Le test `scripts/companion-addon.test.mjs` exécute le vrai fichier Lua dans Fengari avec une API WoW 3.3.5a simulée. Il couvre le premier lancement, la persistance, l’icône de minimap, les positions, les langues, la composition du groupe, les contrôles désactivés, les commandes Playerbots bornées et le remplacement des compagnons hors ligne. Il valide aussi le XML et le manifeste `.toc`. Ce harnais prouve les transitions de l’addon contre des effets factices, pas son rendu ou ses réponses dans OpenWoW.

Ces tests utilisent des effets factices. Ils ne prouvent ni le build Docker complet, ni l'extraction depuis une copie réelle, ni la connexion OpenWoW, ni les bots en jeu. Ces éléments exigent un smoke test manuel documenté séparément.

## Harnais de QA visuelle

Le serveur Vite de développement accepte un état purement visuel avec `?previewState=checking`, `installing`, `ready`, `running` ou `error`. Ce harnais n’est lu que lorsque `import.meta.env.DEV` est vrai et que l’API Tauri est absente. Il ne modifie donc ni le bundle de production ni le comportement desktop. La recette du launcher se fait au viewport fixe de 1024 × 640 ; le site reste testé séparément sur ses tailles responsive.

Ces états servent uniquement à contrôler la composition, le clavier et les états visuels du launcher. Ils constituent une **preuve visuelle simulée**, jamais une preuve d’installation, de progression, de démarrage du serveur ou de jeu réel. Un test Node contrôle en plus que `tauri.conf.json` conserve exactement la fenêtre fixe 1024 × 640, non redimensionnable, non maximisable et centrée.
