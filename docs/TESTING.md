# Tests

`pnpm verify` exécute typecheck, lint, Vitest, build Vite, formatage, Clippy et tous les tests Rust.

Le lanceur actuel teste la sélection des données possédées, le déclenchement de l'installation, le rendu d'un second lancement déjà démarré, le refus UI de l'IA sur une machine non recommandée, la validation minimale du dossier `Data`, les pins et ports de la composition Docker, le preset Ollama local et borné, l'allowlist de modèles, le payload matériel CanIRun, le vecteur SRP6 du compte local, la frontière de commandes système et l'arrêt automatique après disparition du client possédé.

Ces tests utilisent des effets factices. Ils ne prouvent ni le build Docker complet, ni l'extraction depuis une copie réelle, ni la connexion OpenWoW, ni les bots en jeu. Ces éléments exigent un smoke test manuel documenté séparément.
