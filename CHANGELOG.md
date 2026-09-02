# Changelog

## 0.1.0-dev

- Remplacement du faux parcours desktop et des images générées par un lanceur code-native inspiré de la structure des lanceurs MMO de l'ère Wrath.
- Ajout du premier installateur macOS arm64 : OpenWoW officiel vérifié, sources serveur et Playerbots épinglées, build Docker et extraction locale des données serveur.
- Ajout de l'état d'installation atomique, du démarrage automatique aux lancements suivants, de l'option Playerbots et de l'arrêt ordonné.
- Ajout d'un compte joueur local idempotent et de tests du calcul SRP6.
- Conservation des crates fake historiques comme banc de tests isolé, hors du flux produit Tauri actuel.
