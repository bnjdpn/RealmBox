# Construction

```sh
pnpm install
pnpm build
cargo build --workspace
cargo xtask doctor
cargo xtask build-launcher
```

Le build ci-dessus produit un lanceur de développement. Sans configuration de release, celui-ci télécharge l'archive OpenWoW vérifiée et construit le serveur Docker depuis les commits déclarés dans `third-party.lock.toml`. Le workflow de publication des images injecte quatre références GHCR immuables dans les bundles joueur ; ce parcours fait uniquement `docker compose pull` et ne compile pas AzerothCore sur la machine du joueur. Les commandes `xtask build-openwow` et `build-server` sont des garde-fous historiques distincts ; elles échouent volontairement plutôt que de prétendre produire le runtime du joueur.
