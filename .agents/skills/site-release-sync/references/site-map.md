# RealmBox : carte du site et de sa release

| Élément | Source / procédure |
| --- | --- |
| Dépôt / URL | `bnjdpn/RealmBox` · [site public](https://bnjdpn.github.io/RealmBox/) |
| Sources | [site/src](../../../../site/src/) : pages Astro `/`, `/fr/`, `/en/`, 404 bilingue, composants, contenu FR/EN, styles ; [site/public](../../../../site/public/) : assets et manifeste public ; `site/dist` est généré |
| Captures | `site/public/assets/launcher-*.webp` : vrai launcher dans les locales concernées. Les paysages illustrés ne sont pas présentés comme captures. Conserver provenance et absence de données joueur |
| Langues/routes | `site/src/pages/index.astro`, `fr/index.astro`, `en/index.astro`, `404.astro` ; `site/src/content/fr.ts` et `en.ts` ; FAQ/support et liens de téléchargement dans les composants ; sitemap public, 404 exclue du sitemap ; [site/README.md](../../../../site/README.md) décrit les sources et les preuves visuelles |
| Disponibilité | [scripts/generate-site-release-manifest.mjs](../../../../scripts/generate-site-release-manifest.mjs) alimente `site/public/release-manifest.json` depuis la release publique et ses assets ; une prerelease, un tag ou un workflow en cours ne prouve pas la présence d'un installateur |
| Validation | `pnpm site:build` ; tests du manifeste dans `scripts/` si son comportement change ; garde et simulations Ruby du skill ; navigateur sur `site/dist` avec les sous-chemins FR/EN |
| Release | [.github/workflows/release.yml](../../../../.github/workflows/release.yml), `pnpm verify`, contrats de provenance et [docs/UPDATES.md](../../../../docs/UPDATES.md). Revue web dans les artefacts de release existants, sans ajouter d'état de publication permanent |
| Catalogue | `bnjdpn/bnjdpn.github.io` lorsque le produit/public de la release y est mentionné ; ne pas y ajouter automatiquement des promesses à partir du tag |
| Publication | [.github/workflows/pages.yml](../../../../.github/workflows/pages.yml) lit les releases, construit Astro et publie seulement `site/dist` après push `main`/dispatch autorisé ; relire ensuite manifeste, assets et pages publiques |

Conserver Docker, la distinction données joueur/runtime et les limites de
qualification par plateforme. Un fichier macOS présent ne prouve pas Windows ;
les assets de la release et l'installation réellement testée sont deux preuves.
Le contrôle du skill et les simulations sont branchés aux workflows Pages et
release ; ils ne remplacent pas la revue FR/EN du contenu ni la vérification de
chaque téléchargement. Lire [le contrat de revue](review.md).
