# 0004 — Images serveur précompilées

Statut : accepté, publication non encore effectuée.

## Décision

Une release RealmBox destinée aux joueurs ne doit pas compiler AzerothCore ni ses modules sur leur machine. La CI construit une seule fois quatre images Linux (`authserver`, `worldserver`, `db-import`, `tools`) pour `linux/amd64` et `linux/arm64`, à partir des commits immuables du manifeste tiers. Le lanceur téléchargera ensuite ces images par digest, jamais par tag flottant.

OpenWoW et Ollama restent téléchargés depuis leurs releases officielles avec vérification SHA-256. Les données 3.3.5a et les sorties d'extraction ne font partie d'aucune image : elles restent fournies et générées localement par le joueur.

Le build Docker depuis les sources demeure un mode de développement et un filet de récupération explicite. Il ne doit pas être le parcours normal d'une release joueur.

## Publication

Le workflow `server-images.yml` sait vérifier les deux architectures sans publication. Sa publication GHCR est une action manuelle. Après publication et audit des notices, les quatre digests multiarchitecture doivent être relevés, inscrits dans `third-party.lock.toml`, puis utilisés dans le Compose produit. Tant que ces digests n'existent pas, RealmBox conserve le build source et ne prétend pas fournir l'installation instantanée.
