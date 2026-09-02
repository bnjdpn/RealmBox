# Dépannage

- Interface seule : exécuter `pnpm dev:fake`; elle doit afficher clairement « Simulation locale ».
- Toolchain : exécuter `cargo xtask doctor`.
- État interrompu : le domaine sait passer par `Error → Recovering → Ready`; l'écran de réparation complet reste à brancher.
- OpenWoW : vérifier CMake 3.24+, Ninja, vcpkg exact et `VCPKG_ROOT` pour les presets CI.
- Données absentes : ne pas contourner la validation et ne télécharger aucun contenu.

