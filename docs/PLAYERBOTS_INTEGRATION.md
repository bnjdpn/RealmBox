# Intégration Playerbots

Le README et le guide officiels du module imposent encore le fork `mod-playerbots/azerothcore-wotlk`, branche `Playerbot`. Les pins actuels sont dans `third-party.lock.toml`. La documentation avertit qu'elle peut être obsolète ; les clés réellement présentes dans `conf/playerbots.conf.dist` du pin ont été utilisées pour les presets.

RealmBox propose 5, 25, 50, 100 ou 150 bots, puis borne la valeur à partir de la mémoire visible par Docker : 5 sous 12 Gio, 50 sous 20 Gio, 100 sous 28 Gio, 150 au-delà. Une valeur inconnue retombe à 5. Les guildes aléatoires sont désactivées pour éviter leur coût mémoire.

Mesure réelle du 2 septembre 2026 : avec 15,8 Gio accordés à Docker, 50 bots autonomes sont restés connectés et `worldserver` s’est stabilisé autour de 5,2 Gio. Quatre bots de groupe supplémentaires ont ensuite été invoqués autour du joueur. Cette mesure valide le palier 50 sur le Mac de développement, pas les paliers 100 et 150.

Le sélecteur du launcher s’applique actuellement au prochain démarrage. La gestion à chaud devra passer par une passerelle bornée vers les commandes Playerbots, sans exposer une console serveur générique à l’interface.
