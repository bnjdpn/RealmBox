# Construction

```sh
pnpm install
pnpm build
cargo build --workspace
cargo xtask doctor
cargo xtask build-launcher
```

Le build ci-dessus produit le lanceur. Le parcours produit télécharge l'archive OpenWoW vérifiée et construit le serveur dans Docker depuis les commits déclarés dans `third-party.lock.toml`. Les commandes `xtask build-openwow` et `build-server` sont des garde-fous historiques distincts ; elles échouent volontairement plutôt que de prétendre produire le runtime du joueur.
