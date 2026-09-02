# RealmBox

RealmBox est une application desktop open source qui vise à transformer une pile locale complexe en une expérience solo simple : fournir une fois ses propres données compatibles, préparer le monde, puis cliquer sur **Jouer**.

RealmBox fournit le lanceur, l'orchestration, les composants open source autorisés et l'addon RealmBox Companions. Il ne fournit ni client propriétaire, ni archives, cartes, textures, sons ou autres données de jeu. Le client visé est [OpenWoW](https://github.com/rkabachenko/OpenWow-snapshot), une réimplémentation open source expérimentale ciblant le protocole et les données 3.3.5a build 12340.

## État actuel

Le vertical slice 0.1 fonctionne avec un runtime fake : onboarding en français, choix d'ambiance, préparation progressive, dashboard, bouton Jouer, groupe de quatre compagnons, conversation simulée, arrêt et persistance SQLite côté Tauri. Le fake est visiblement étiqueté et utilise les mêmes interfaces Rust que les backends réels.

Le parcours réel OpenWoW → serveur → Playerbots → Ollama n'est **pas encore validé**. Aucun contenu propriétaire n'est présent. Voir [STATUS.md](STATUS.md) pour les preuves exactes.

## Développement macOS

Prérequis : Rust 1.97.1, Node 25+, pnpm 10+, Xcode. Puis :

```sh
pnpm install
pnpm dev:fake
pnpm verify
cargo xtask doctor
```

`pnpm dev:fake` ouvre l'interface dans le navigateur sans services réels. `pnpm dev` démarre Tauri et persiste l'état dans le répertoire applicatif standard. Les commandes de build upstream refusent explicitement de prétendre à un succès tant que les runtimes ne sont pas prêts.

## Plateformes visées

- macOS arm64 : cible de développement principale, fake testé automatiquement.
- Windows x86-64 : workflows préparés, exécution réelle requise sur runner Windows.
- macOS x86-64 : cross-build préparé, stabilité interdite sans smoke test sur Mac Intel.
- Linux : hors périmètre 0.1.

RealmBox est licencié sous AGPL-3.0-only, choix conservateur motivé dans [ADR 0001](docs/decisions/0001-license.md). Ce choix et la redistribution de la pile doivent encore recevoir une revue juridique avant publication binaire.

