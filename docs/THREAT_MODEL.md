# Modèle de menaces

Actifs : secrets locaux, personnages, sauvegardes, données de jeu, intégrité des runtimes et confidentialité des conversations. Frontières : fichiers utilisateur, archives téléchargées, manifestes, services localhost, modules C++ et client.

Menaces prioritaires : traversal/Zip Slip, symlinks, artefacts falsifiés ou partiels, injection d'arguments, port détourné, PID réutilisé, secret journalisé et suppression de monde pendant réparation. Les vérifications de hash/signature, commits, ports loopback, écriture atomique et commandes typées sont implémentées. Le ZIP est extrait par `ditto` après hash mais son inventaire n'est pas encore prévalidé ; l'identité du PID client n'est pas persistée après crash. Ces deux limites restent bloquantes pour une release.
