# Avis relatifs aux composants tiers

RealmBox est distribué sous AGPL-3.0-only. Ce fichier inventorie les composants
upstream prévus ou évalués; il ne prétend pas qu'ils sont déjà distribués dans
un paquet RealmBox.

| Composant | Révision | Licence déclarée | Inclusion actuelle |
|---|---|---|---|
| OpenWoW snapshot | `2521e1f72eeffd661913be63d1e2b374073c316c` | AGPL-3.0-only | source non vendored; build de compatibilité uniquement |
| AzerothCore Playerbot fork | `47960183bb03b83e8943eb2f0f39c16df9710c9d` | GPL-2.0-only | source non vendored; build de compatibilité uniquement |
| mod-playerbots | `2f7d9f774987d0157c6a0d0cc08c40bec3db3945` | GPL-2.0-only | source non vendored; build de compatibilité uniquement |
| mod-ollama-chat | `a9d14b0b8955be136e657ac168dd255f5281a535` | AGPL-3.0-only | source non vendored; build de compatibilité uniquement |
| Ollama | `e5e437711540eb4becb393c2847fed6cae6e5cd5` | MIT | non intégré et non redistribué |

Les textes de licence conservés dans `LICENSES/` proviennent des révisions
épinglées correspondantes. Les bibliothèques Rust et npm sont résolues dans
`Cargo.lock` et `pnpm-lock.yaml`; les rapports SBOM et licence sont produits par
la CI de validation. Les données, médias et binaires propriétaires du jeu ne
sont jamais fournis par RealmBox.

La décision de redistribuer un binaire upstream reste bloquée tant que ses
dépendances transitives, notices, sommes de contrôle, signature et provenance
de build n'ont pas été auditées et publiées.
