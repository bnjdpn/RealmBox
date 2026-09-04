# Construction

```sh
pnpm install
pnpm build
cargo build --workspace
cargo xtask doctor
cargo xtask build-launcher
```

Le build ci-dessus produit un lanceur de développement. Sans configuration de release, celui-ci télécharge l'archive OpenWoW vérifiée et construit le serveur Docker depuis les commits déclarés dans `third-party.lock.toml`. Le workflow de publication des images injecte quatre références GHCR immuables dans les bundles joueur ; ce parcours fait uniquement `docker compose pull` et ne compile pas AzerothCore sur la machine du joueur. Les commandes `xtask build-openwow` et `build-server` sont des garde-fous historiques distincts ; elles échouent volontairement plutôt que de prétendre produire le runtime du joueur.

`pnpm verify` compile un petit harnais C++17 qui exécute la politique de coupe-circuit extraite du patch Ollama exact. Il nécessite `c++` sur macOS/Linux ou MSVC `cl.exe` sur Windows. Le contrôle SQL réel est volontairement séparé parce qu’il exige Docker : `pnpm test:guide-sql` démarre un MySQL épinglé, isolé et jetable, sans réseau, port, volume persistant ni donnée de jeu.

## Images et installateurs d’une release

Le workflow `Release` résout une fois le commit du tag puis appelle `server-images.yml` pour reconstruire les quatre images sur les deux architectures depuis ce commit. Les noms temporaires des images incluent le commit RealmBox, l’identifiant d’exécution et la tentative : deux constructions ne peuvent pas mélanger leurs images via un tag amont partagé.

Les installateurs attendent les manifestes multiarchitectures et le contrôle des quatre références `@sha256`. Ils consomment uniquement les sorties de cette construction, jamais les variables d’images du dépôt. Un échec de construction, d’accès anonyme ou de validation des références empêche leur publication. Les fichiers `release-images.env` et `release-source.txt` sont joints à la release avec les installateurs et `SHA256SUMS.txt` pour rendre cette provenance inspectable.

Pour reprendre une release existante sans déplacer son tag, lancer le workflow corrigé depuis `main` avec `gh workflow run release.yml --ref main -f tag=v0.5.0` (adapter le tag). Le code de l’application, le patch serveur et les captures proviennent du commit du tag ; seule l’orchestration provient de `main`. Le tag est revérifié avant l’ajout des fichiers. La visibilité publique d’une release existante n’est pas modifiée. Une nouvelle release absente reste créée en brouillon par défaut.

Le lancement autonome de `server-images.yml` peut encore produire des bundles de développement, mais ne les joint pas à une release. Il n’est pas un raccourci de publication. Les tests `scripts/release-workflow.test.mjs` couvrent les dépendances et le refus des références absentes, mutables ou issues d’une autre source/exécution. `actionlint .github/workflows/release.yml .github/workflows/server-images.yml` vérifie la syntaxe et les expressions GitHub Actions lorsqu’il est disponible.
