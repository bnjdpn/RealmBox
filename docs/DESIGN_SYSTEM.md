# Design system RealmBox

## Principes de marque

RealmBox représente un monde fantasy gelé contenu dans une machine locale. Le launcher doit donner trois informations immédiates : l’univers, l’état courant et l’action suivante. Il occupe une fenêtre fixe de 1024 × 640 et se lit comme un launcher PC, jamais comme un site, un tableau de bord ou un back-office. Le site reste responsive et éditorial ; il ne dicte pas la composition de l’application desktop.

Le produit reste indépendant. L’identité évite le globe, le W, les armes, les casques, les crânes, les blasons de faction et toute rune provenant d’une franchise existante.

## Tokens fondamentaux

| Token | Valeur | Usage |
| --- | --- | --- |
| `--abyss` | `#050b12` | fond le plus profond |
| `--night` | `#071725` | fonds principaux |
| `--steel-dark` | `#10293a` | plaques et cadres |
| `--steel` | `#1c4358` | reliefs et survols |
| `--ice-deep` | `#326e8d` | progression et halos profonds |
| `--ice` | `#9ed2e5` | contrôles et filets froids |
| `--ice-light` | `#d8f2f7` | focus et reflets |
| `--bronze` | `#5a3e21` | ombres chaudes |
| `--brass` | `#98723a` | bordures actives |
| `--old-gold` | `#c7a65a` | titres secondaires et action |
| `--gold-light` | `#e7d293` | texte de bouton et focus chaud |
| `--parchment` | `#d7c39c` | guide et contenu éditorial |
| `--ink` | `#221a13` | texte sur parchemin |
| `--success` | `#7fa46b` | état prêt/running, avec forme ou libellé |
| `--error` | `#8e3037` | erreur, toujours accompagnée de texte et d’un cadre |

Les tokens essentiels sont dupliqués explicitement dans `apps/desktop/src/styles.css` et `site/assets/site.css`. Cette duplication évite de créer un pipeline partagé entre Vite et le site statique ; toute modification de palette doit mettre les deux listes à jour.

## Composition du launcher

- L’illustration ImageGen remplit toute la WebView avec un recadrage `cover` ; aucun cadre interne ne double la fenêtre native.
- Le logo RealmBox apparaît une seule fois, dans le coin supérieur gauche.
- L’état et l’action principale forment un seul groupe en bas à gauche. Il n’existe ni navigation permanente, ni grille de composants, ni rangée de badges.
- Le bouton Réglages est le seul contrôle secondaire permanent. Il ouvre un panneau latéral unique pour la langue, l’installation, les compagnons, les dialogues et le diagnostic.
- Les séparateurs du panneau restent neutres et fonctionnels. Les bordures dorées, doubles filets, losanges et glyphes décoratifs sont interdits dans l’interface desktop.
- La progression n’est visible qu’entre 1 et 99 % pendant une installation, un démarrage ou un arrêt. Les états stables n’affichent aucune jauge.
- Une erreur affiche une cause courte, une récupération et une action. Les journaux, chemins et détails bruts restent dans Diagnostic.

## Espacement

Échelle recommandée : 4, 8, 12, 18, 24, 32 et 48 px. Les groupes interactifs conservent au moins 40 px de hauteur lorsque la place le permet. Les panneaux denses utilisent des séparateurs plutôt que des marges excessives.

## Typographie

- Titres et wordmark : `Georgia`, puis `Times New Roman`, serif. Ces polices système gèrent le français et l’anglais sans téléchargement.
- Corps et contrôles : `Segoe UI`, Inter si déjà disponible localement, puis pile système.
- Journaux et chemins : `ui-monospace`, `SFMono-Regular`, Consolas, monospace.
- Aucun texte fonctionnel n’est rasterisé dans une image.
- Les petits textes décoratifs restent secondaires ; les instructions, erreurs et actions ne descendent pas sous une taille lisible dans leur contexte.

## États interactifs

- Hover : éclaircissement mesuré du métal ou du laiton.
- Focus : contour `--ice-light` de 3 px, décalé de 3 px ; il reste visible indépendamment de la couleur de fond.
- Pressed : translation verticale de 1 px et ombre interne plus forte.
- Disabled : baisse d’opacité et de saturation, curseur neutre, libellé inchangé.
- Selected : fond froid contrasté et `aria-pressed=true` dans le sélecteur de langue des réglages ; `aria-pressed` dans le sélecteur de langue du site.
- Running : état EN JEU/IN GAME stable ; l’arrêt demeure une action secondaire séparée.

## Mouvement

Les transitions du launcher sont courtes et fonctionnelles : ouverture du panneau, survol, focus et progression active. Aucun son, flash, parallax, neige procédurale, vidéo ou WebGL.

Sous `prefers-reduced-motion: reduce`, les animations et transitions sont ramenées à un seul état quasi instantané.

## Icône

Le médaillon associe un portail cyan, une coque acier/laiton et un monogramme R/B abstrait. Sa forme extérieure polygonale et son anneau épais forment la silhouette à 16 et 32 px. La version monochrome conserve uniquement la coque, les deux anneaux et le monogramme.

- Conserver une marge de sécurité de 36 px sur la source 1024 px.
- Ne pas poser l’icône dans un cercle ou un globe additionnel.
- Ne jamais ajouter de petit texte.
- Employer la version couleur pour l’application, le favicon et les panneaux de marque.
- Employer la version monochrome pour l’impression, les masques ou un contexte à une seule couleur.

## Accessibilité

- Contraste cible de 4,5:1 pour tout texte courant.
- Ordre DOM identique à l’ordre visuel.
- Tous les contrôles ont un nom accessible et un focus visible.
- Les changements d’état du launcher restent dans sa région `aria-live`.
- Aucun état n’est transmis par la couleur seule : forme, libellé ou position complète l’information.
- Le site propose un lien d’évitement et des repères sémantiques.
- Les cibles principales dépassent 40 px et restent utilisables au zoom.

## Launcher et site

Le launcher concentre l’action dans une scène fixe et relègue les fonctions avancées dans un panneau contextuel. Le site déploie la même identité dans une composition éditoriale responsive. Les deux surfaces partagent la palette, l’icône et les panoramas originaux, mais pas leur structure d’information.
