# Release

Une release exige : suite verte, SBOM, audit licences, manifestes/checksums, artefacts par architecture, smoke tests séparés, signature conditionnelle et lecture fraîche des artefacts. Sans certificat, l'artefact doit être annoncé non signé. Aucun statut universel ou stable sans preuve pour chaque architecture.

Le workflow `release.yml` réagit aux tags `v*`. Il refuse de construire si les quatre variables de dépôt `REALMBOX_AUTH_SERVER_IMAGE`, `REALMBOX_WORLD_SERVER_IMAGE`, `REALMBOX_DB_IMPORT_IMAGE` et `REALMBOX_TOOLS_IMAGE` ne sont pas des références GHCR complètes par digest SHA-256. Après validation, il produit un DMG macOS arm64 et un installateur NSIS Windows x64, puis les joint à une prerelease GitHub laissée en brouillon pour relecture humaine.

Le brouillon ne doit être publié qu’après :

- audit frais des notices et SBOM ;
- téléchargement anonyme des quatre images ;
- smoke manuel sur chaque architecture annoncée ;
- signature/notarisation, ou libellé explicite `unsigned` ;
- confirmation qu’aucune donnée propriétaire n’est présente.
