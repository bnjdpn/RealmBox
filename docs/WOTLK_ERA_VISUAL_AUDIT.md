# Audit visuel des portails MMORPG 2008–2010

## Cadre de l’étude

Cet audit étudie des signatures de composition et d’ergonomie de portails et lanceurs de MMORPG PC de 2008–2010. Il ne constitue pas un inventaire d’assets à reproduire. RealmBox ne réutilise aucune capture, illustration, typographie, rune, bordure, logo, personnage, architecture ou ressource de Blizzard.

Références consultées le 3 septembre 2026 :

- [actualités Battle.net d’août 2008](https://classic.battle.net/news/0808.shtml), pour la densité éditoriale, les accès aux médias et le rôle de portail d’actualité ;
- [annonce du changement d’habillage du site officiel en novembre 2008](https://www.engadget.com/2008-11-10-official-wow-site-now-full-wrath.html), pour la forte adaptation saisonnière du décor ;
- [documentation historique de la version 3.3.5a](https://warcraft.wiki.gg/wiki/Patch_3.3.5a), uniquement pour dater la build 12340 au 29 juin 2010 ;
- [rappel communautaire des anciens sites officiels](https://eu.forums.blizzard.com/en/wow/t/check-out-the-old-wow-websites/160670), pour confirmer le caractère riche, éditorial et multi-module de cette période ;
- [témoignages sur le launcher historique](https://www.reddit.com/r/wow/comments/sjxiqu), utilisés uniquement comme indices ergonomiques sur la prééminence du bouton de lancement et de la progression.

Les observations ont été synthétisées sans télécharger ni intégrer les images des pages référencées.

## Matrice de réinterprétation

| Signature historique observée | Ce qui la rend reconnaissable | Adaptation originale RealmBox | Asset ou technique | Contrôle de propriété intellectuelle |
| --- | --- | --- | --- | --- |
| Grand panorama saisonnier | L’illustration occupe une part majeure du lanceur et installe le monde avant les contrôles | Vallée nocturne originale avec montagnes, portail circulaire et architecture monolithique | Deux illustrations raster distinctes générées par ImageGen sans image d’entrée, documentées dans `docs/ASSET_PROVENANCE.md` | Aucun personnage, casque, arme, trône, citadelle ou décor officiel |
| Fenêtre comme objet du monde | Le cadre semble être une plaque ou un meuble ouvragé, pas une page web neutre | Cadre RealmBox en acier sombre, double filet de laiton, attaches angulaires | CSS, doubles bordures, ombres internes et biseaux | Géométrie et proportions créées pour RealmBox |
| Navigation compacte en plaques | Peu d’entrées, intégrées au cadre, avec sélection très marquée | Plaques horizontales Mon monde, Compagnons, Dialogues et Diagnostic | CSS, états `aria-pressed`, focus clair | Aucun sprite ou bouton extrait d’un jeu |
| Action de lancement dominante | Grand bouton placé au bord inférieur, immédiatement identifiable | Bouton JOUER/PLAY rectangulaire en laiton ancien avec double encadrement | CSS, relief, états hover/focus/pressed/disabled | Aucun emblème, lettrage ou forme de bouton propriétaire |
| Progression persistante | Le patcher conserve une lecture continue de l’opération | Barre lumineuse basse, pourcentage réel et composants vérifiés | Données runtime existantes et animation CSS | Aucun débit ni délai fictif |
| Panneau d’actualités et d’état | L’information éditoriale voisine avec l’action principale | Site à trois colonnes avec téléchargements, guide central et état factuel | HTML sémantique et CSS Grid | Contenu RealmBox uniquement |
| Surfaces de parchemin | Les guides longs sont différenciés des panneaux système | Guide d’installation beige, encre sombre, chapitres numérotés | Gradients CSS et bordures locales | Texture procédurale sans scan ni texture tierce |
| Métal froid et accents chauds | Contraste acier/bleu et or vieilli | Palette acier, glace cyan et laiton définie dans les tokens | Variables CSS documentées | Palette générique, sans échantillonnage d’un asset protégé |
| Ornements pseudo-runiques | Petits signes structurent la hiérarchie | Losanges abstraits, anneaux et traits géométriques propres à RealmBox | SVG et pseudo-éléments CSS | Aucun alphabet ou glyphe Warcraft reproduit |
| Wordmark monumental | Fort contraste, sérif, relief et ombre | REALM / BOX en capitales système Georgia, composition rectangulaire libre | Texte HTML/CSS, jamais intégré à l’image | Ni globe, ni ovale, ni W, ni police Warcraft |
| Monde gelé mis en scène | Brume, ciel nocturne et lumière froide donnent la tonalité | Lumière provenant du portail, vallée vide et neige rare | Raster ImageGen optimisé en WebP ; pas de halo ni de neige simulés par-dessus | Paysage entièrement original, sans référence visuelle fournie au modèle |
| Erreur intégrée au thème | L’avertissement demeure dans le même objet visuel | Avis scellé brun/acier avec cause, récupération et accès au diagnostic | CSS, texte runtime, forme non propriétaire | L’état n’est pas transmis par le rouge seul |

## Décisions retenues

- La composition reprend le rythme d’un portail de jeu PC de la période, mais aucun élément n’est calqué sur un écran historique particulier.
- Le panorama précédent a été retiré : sa provenance n’était pas documentée et sa silhouette centrale risquait d’évoquer trop directement une œuvre protégée.
- Les détails techniques restent dans Diagnostic. La densité visuelle vient du cadre, des surfaces et de la hiérarchie, pas d’une multiplication de réglages serveur.
- Les animations sont limitées au halo, à la neige rare, à la progression et aux états de transition, avec une variante statique via `prefers-reduced-motion`.
