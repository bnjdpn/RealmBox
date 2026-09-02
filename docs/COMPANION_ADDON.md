# RealmBox Companions

Le squelette 0.1 utilise uniquement des boutons associés à une table de commandes constante. Aucune saisie utilisateur n'est concaténée à une commande Playerbots. Les commandes `follow`, `attack`, `stay`, `summon` et `leave` sont documentées dans le wiki du projet Playerbots consulté le 2 septembre 2026. `cooldowns on` reste à vérifier dans le commit épinglé avant activation en parcours réel.

Cette première version envoie les commandes fixes sur le canal de groupe. Elle ne fournit pas encore de passerelle serveur authentifiée, de sélection de composition, ni de dialogue avec `mod-ollama-chat`. Le panneau sert de squelette testable visuellement lorsque l'API addon OpenWoW est disponible avec des données utilisateur.

Test manuel : copier le dossier dans `Interface/AddOns`, vérifier que l'addon est listé, entrer dans un groupe de bots, puis tester chaque bouton avec les logs Playerbots ouverts. Ne pas considérer ce test comme validé tant que les réponses du serveur ne sont pas observées.
