# Tests

`pnpm verify` exécute typecheck, lint, Vitest, build Vite, formatage, Clippy et tous les tests Rust.

Le lanceur actuel teste la sélection des données possédées, le déclenchement de l'installation, le rendu d'un second lancement déjà démarré, la validation minimale du dossier `Data`, les pins et ports de la composition Docker, le vecteur SRP6 du compte local et la frontière de commandes système.

Ces tests utilisent des effets factices. Ils ne prouvent ni le build Docker complet, ni l'extraction depuis une copie réelle, ni la connexion OpenWoW, ni les bots en jeu. Ces éléments exigent un smoke test manuel documenté séparément.
