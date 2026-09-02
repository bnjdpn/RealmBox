# Construction

```sh
pnpm install
pnpm build
cargo build --workspace
cargo xtask doctor
cargo xtask build-launcher
```

OpenWoW et le serveur sont construits depuis des clones temporaires ou un cache de sources vérifié. Les futurs `xtask build-openwow` et `build-server` doivent vérifier le commit, appliquer un patchset explicite, construire, tester puis produire un manifeste. Ils échouent volontairement aujourd'hui plutôt que de produire un faux artefact.

