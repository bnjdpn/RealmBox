# Provenance des assets visuels

Tous les assets listés ci-dessous ont été créés spécifiquement pour RealmBox le 3 septembre 2026. Ils ne proviennent d’aucun jeu, wiki, fan site, archive web, CDN, modèle 3D, texture ou capture. Les illustrations ont été générées sans image d’entrée ni référence visuelle fournie au modèle. Elles sont distribuées avec le dépôt sous AGPL-3.0-only.

| Chemin | Fonction | Auteur / méthode | Dimensions | Licence | Contrôle de contenu propriétaire |
| --- | --- | --- | --- | --- | --- |
| `apps/desktop/src/assets/launcher-hero-bg.webp` | fond principal du launcher | OpenAI ImageGen intégré, puis conversion WebP locale avec ImageMagick | 1672 × 941 | AGPL-3.0-only | Monde, portail et architecture originaux ; aucun personnage, texte, logo, arme, crâne, rune lisible ou élément de franchise |
| `site/public/assets/hero-azeroth-companions.webp` | panorama principal du site | OpenAI ImageGen intégré, puis conversion WebP locale avec `cwebp` | 1672 × 941 | AGPL-3.0-only | Monde, cité, costumes et groupe de cinq aventuriers originaux ; aucun personnage, race, faction, symbole, bâtiment ou asset de franchise |
| `site/public/assets/hero-realmbox.webp` | ancien panorama glacé conservé dans les assets du site | OpenAI ImageGen intégré, génération distincte, puis conversion WebP locale avec ImageMagick | 1915 × 821 | AGPL-3.0-only | Composition et citadelle distinctes du launcher ; aucun élément de franchise |
| `site/public/assets/og-realmbox.webp` | carte sociale | Recadrage 1200 × 630 et conversion WebP locale du panorama de site généré | 1200 × 630 | AGPL-3.0-only | Aucun élément ajouté, aucun texte rasterisé |
| `branding/realmbox-icon.svg` | source couleur 1024 × 1024 | Médaillon, anneaux et monogramme R/B dessinés en SVG local | 1024 × 1024 | AGPL-3.0-only | Pas de W, globe, casque, épée, crâne, blason ou rune de franchise |
| `branding/realmbox-icon-monochrome.svg` | variante monochrome | Simplification géométrique de l’icône couleur | 1024 × 1024 | AGPL-3.0-only | Même contrôle que l’icône couleur |
| `branding/realmbox-icon-1024.png` | source raster Tauri | Rendu local ImageMagick de l’icône SVG, transparence conservée | 1024 × 1024 | AGPL-3.0-only | Aucun nouvel élément ajouté |
| `apps/desktop/src/assets/realmbox-icon.svg` | marque dans le launcher | Copie SVG optimisée de l’icône couleur | vectoriel | AGPL-3.0-only | Aucun nouvel élément ajouté |
| `apps/desktop/public/favicon.svg` | favicon de l’aperçu Vite | Copie SVG optimisée de l’icône couleur | vectoriel | AGPL-3.0-only | Aucun nouvel élément ajouté |
| `site/public/assets/icon.svg` | favicon et marque du site | Copie SVG optimisée de l’icône couleur | vectoriel | AGPL-3.0-only | Aucun nouvel élément ajouté |
| `site/public/assets/launcher-*.webp` | captures du launcher prêt, installation, compagnons et dialogues | Playwright sur le harnais Vite purement visuel, sans API Tauri ni donnée de jeu | 1024 × 640 | AGPL-3.0-only | Interface RealmBox et panorama original uniquement ; états simulés explicitement identifiés |
| `apps/desktop/src-tauri/icons/*` | icônes macOS, Windows, iOS et Android | Génération locale par `pnpm --dir apps/desktop tauri icon ../../branding/realmbox-icon-1024.png` | multi-format | AGPL-3.0-only | Génération de formats uniquement |

## Prompts ImageGen

### Launcher

> Illustration d’environnement dark-fantasy gelé, nocturne et originale, destinée au fond d’un launcher. Vallée arctique, montagnes distantes, portail circulaire monumental en pierre glaciaire noire placé dans le tiers droit, énergie cyan retenue et citadelle monolithique lointaine. Composition 16:9 avec 38 % de calme sombre à gauche pour le mot-symbole HTML. Matériaux réalistes, profondeur atmosphérique, palette charbon, bleu nuit et acier. Aucun personnage, créature, arme, crâne, corne, texte, lettre, logo, filigrane, UI, symbole copié ni imagerie de franchise ; gravures géométriques abstraites et non linguistiques. Éviter le violet néon, la symétrie centrée, les flèches de château génériques, le rendu 3D brillant et la typographie intégrée.

### Site — panorama principal

> Illustration 16:9 high-fantasy MMO originale, peinte à la main, destinée au hero du site. Une route traverse une frontière verdoyante vers une immense cité fortifiée originale ; montagnes bleues, forêt de conifères, lumière dorée et accents arcaniques teal. Exactement cinq aventuriers vus de dos se dirigent vers la cité : un joueur en tête et quatre compagnons aux silhouettes distinctes. Garder 40 % de calme sombre à gauche pour le texte HTML et une composition recadrable sur mobile. Aucun texte, logo, filigrane, portail circulaire, personnage, race, faction, arme, bâtiment, géographie, UI, typographie ou asset reconnaissable de Warcraft, World of Warcraft, Blizzard ou d’une autre franchise. Éviter le rendu 3D brillant, le lissage artificiel, la photoréalité, la science-fiction, les duplications et les figures supplémentaires.

### Ancien panorama du site

> Panorama 21:9 d’un royaume dark-fantasy gelé, nocturne et original, destiné au hero d’un site. Immense vallée arctique, portail ancien plus petit dans le paysage au tiers droit, montagnes superposées et citadelle originale aux tours basses et massives. Grande zone sombre à gauche et en haut pour le contenu HTML, lumière cyan discrète, rares fenêtres ambre, pierre et glace crédibles. Aucun personnage, créature, arme, crâne, corne, texte, lettre, logo, filigrane, UI, symbole copié ni imagerie de franchise ; gravures abstraites non linguistiques. Éviter le néon, le plein jour, les armées, dragons, rochers flottants, ornements dorés et la typographie intégrée.

Les prompts complets exécutés reprenaient ces contraintes sous forme de brief structuré (usage, scène, composition, lumière, palette, matériaux, contraintes et éléments à éviter).

## Assets remplacés

- `site/assets/realm-frost.webp` et `apps/desktop/src/assets/frostbound-realm-v2.webp` : anciens rasters sans provenance documentée et à silhouette ambiguë.
- `branding/realmbox-panorama.svg`, `apps/desktop/src/assets/realm-panorama.svg` et `site/assets/realm-panorama.svg` : décors procéduraux CSS/SVG de transition, supprimés après intégration des illustrations ImageGen.
- Les halos de portail et la neige simulés en CSS ont également été retirés. Le cadre, les ombres de lisibilité et les composants d’interface restent en CSS.

## Poids et formats

Les fichiers WebP principaux pèsent environ 164 Kio pour le launcher, 226 Kio pour le panorama actuel du site et 128 Kio pour la carte sociale. Aucune police, texture ou illustration n’est chargée depuis un domaine tiers. Aucune texture ImageGen séparée n’a été ajoutée : les surfaces de pierre, métal, parchemin et lumière restent déterministes en CSS.
