# Direction visuelle

## Intention

RealmBox doit rappeler le calme, l'échelle et la camaraderie d'une aventure
fantasy en ligne classique sans reprendre une œuvre, un personnage, une race,
un lieu, une interface, un logo ou une silhouette appartenant à Blizzard. La
direction retenue est une illustration éditoriale à la gouache sèche, aux
formes simples et à la palette restreinte: vert lichen, pin charbon, bleu
ardoise, parchemin sourd et petites touches d'ocre.

## Garde-fous anti-slop

La recherche préalable a retenu cinq critères: art direction spécifique plutôt
qu'un style générique; composition asymétrique; texture et imperfections
matérielles; palette courte; suppression de tout effet sans fonction. Cela évite
notamment les gradients violet-cyan, le bloom, les portails magiques, les héros
alignés, le micro-détail uniforme et les surfaces 3D plastiques. Cette approche
rejoint l'analyse de [Creative Bloq](https://www.creativebloq.com/ai/everything-looks-the-same-now-what)
sur la convergence vers une moyenne visuelle et la recommandation [Adobe/EY](https://business.adobe.com/content/dam/dx/us/en/resources/reports/leading-generative-ai-deployment-for-marketing/EY_GenAI_Guide_2024_Interactive.pdf)
de vérifier que le contenu génératif enrichit réellement la création et reste
fidèle à une direction de marque.

Les images n'embarquent ni texte ni logo. La typographie, les bordures et les
contrastes restent du code déterministe. Chaque asset répond à un usage précis:
le paysage donne un horizon à l'accueil; le portrait remplace un placeholder
alphabétique dans le dashboard.

## Prompts finaux

### Paysage d'accueil

Illustration fantasy éditoriale originale pour un lanceur desktop: vallée de
montagne au crépuscule bleu, balises de sentier en pierre sombre appartenant à
une culture entièrement originale, pins courbés par le vent, petit feu ambré et
abri de toile, observatoire rond lointain. Gouache et brosse sèche sur papier
légèrement grainé, masses simplifiées, composition large asymétrique avec
l'intérêt dans les deux tiers droits et trois plans de profondeur. Palette vert
lichen, pin charbon, bleu ardoise, parchemin sourd et ocre. Aucun personnage
proche, texte, logo, rune, interface, portail cyan, armure, motif de franchise,
ressemblance Warcraft/Blizzard, bloom excessif, plastique 3D ou spectacle
fantasy générique.

### Portrait du voyageur

Portrait carré original d'un voyageur humain protecteur adulte, épaules et tête
en trois-quarts, expression calme, laine et cuir usés, petite attache de cape en
laiton, cheveux courts sombres avec une mèche grise. Même gouache sèche sur
papier, cadrage asymétrique lisible dans un cercle de 150 px, lumière latérale
fraîche et reflet chaud très discret. Aucun texte, logo, arme, main, héraldique,
yeux lumineux, oreilles pointues, épaulières, silhouette reconnaissable,
ressemblance Warcraft/Blizzard, retouche beauté, pose glamour, orange-teal,
filigrane doré ou rendu 3D plastique.

## Validation

Les deux sorties ont été inspectées avant intégration, converties en WebP puis
vérifiées dans le vrai frontend avec Playwright à 1200 × 920 et 390 × 844. La
provenance et les sommes de contrôle sont dans
`apps/desktop/src/assets/README.md`.
