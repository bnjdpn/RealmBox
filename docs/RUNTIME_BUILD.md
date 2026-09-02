# Construction des runtimes

Chaque runtime devra provenir d'un commit ou artefact exact, être vérifié avant exécution, rester natif pour l'architecture annoncée et contenir un manifeste, les checksums et les notices. Les données extraites de l'utilisateur sont exclues de tout package.

Les commandes `xtask build-openwow`, `build-server`, `build-runtimes` et `package` sont actuellement des garde-fous bloquants tant que ce pipeline n'est pas implémenté.

