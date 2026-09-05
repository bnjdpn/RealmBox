# Site RealmBox

Sources Astro : `src/pages/` (FR, EN, redirection racine et 404),
`src/content/fr.ts` et `en.ts`, composants dans `src/components/`, styles dans
`src/styles/global.css`. Assets locaux et sitemap dans `public/` ; sorties
régénérables dans `dist/`. Aucune donnée de jeu ni donnée utilisateur à ajouter.

`pnpm site:build` depuis la racine construit le site statique sous `/RealmBox/`.
Le workflow `.github/workflows/pages.yml` publie `site/dist` après génération
du manifeste public ; ni le code Astro, ni les guides internes ne sont copiés.
Un build local n'est pas une publication. Conserver tous les liens de section.

L'ouverture montre une vraie vue du launcher 0.5.0 avec données de démonstration,
identifiée dans sa légende. Les fichiers `launcher-*-fr.webp` et `*-en.webp`
proviennent du workflow `scripts/capture-launcher.mjs` ; leur provenance est
documentée dans `docs/ASSET_PROVENANCE.md`. L'illustration demeure un asset de
partage, pas une preuve de jeu. Aucun visuel synthétique ne remplace une capture.

Les liens de téléchargement renvoient à la **préversion**. La présence d'un DMG
ou EXE n'établit ni signature, ni notarisation, ni qualification complète en jeu.
Le manifeste public et les textes doivent rester cohérents par plateforme.

La [carte de maintenance](../.agents/skills/site-release-sync/references/site-map.md)
décrit le contrôle obligatoire lors d'un changement produit. Vérifier les trois
formats 390, 768 et 1440 px, les menus Entrée/Échap, FR/EN et les liens sortants.
La 404 est bilingue et exclue du sitemap. Relire l'URL réellement servie après
une publication autorisée, séparément du build et de la CI.
