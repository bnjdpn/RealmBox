---
name: site-release-sync
description: Keep this product's public site and portfolio accurate when features, UI, screenshots, platforms, offers or data handling change, including release preparation and public release readback.
---

# Maintenir le site avec le produit

Lire [la carte locale](references/site-map.md) : sources réellement éditables,
routes, langues, captures, contrôles et publication. Ce skill et ses scripts
sont autonomes dans le clone ; ne pas chercher un dossier parent du portefeuille.

- Comparer le changement au produit **publiquement téléchargeable par plateforme**.
  Code, captures de développement, métadonnées préparées, merge, tag et soumission
  au store ne prouvent pas la disponibilité. Relire le store public ou les fichiers
  de release effectivement téléchargeables ; conserver la preuve exacte dans
  les artefacts de release existants, avec version, plateforme, URL et date.
- Vérifier présentation, aide/FAQ, support, confidentialité, conditions, captures,
  compatibilité, offre et catalogue principal selon l'impact. Maintenir FR/EN,
  les autres traductions existantes et les URL store/sous-chemins. Conserver le
  sens juridique, les avertissements et les protections de formulaire.
- Modifier les sources, puis régénérer. Une UI qui rend les visuels inexacts
  exige de nouvelles captures réelles avec données de démonstration et provenance.
  Préparer ces captures et textes avec le candidat. Tant que sa release est en
  attente, conserver le contenu public confirmé et garder les nouveautés dans
  une branche/patch non publié, hors de l'artefact Pages courant. Ne pas brancher
  automatiquement les métadonnées du candidat sur des promesses publiques.
- Un refactoring interne peut conclure « aucun impact public », avec une raison
  précise ; ne pas créer artificiellement des modifications de site. Si le
  catalogue est touché, traiter aussi `bnjdpn/bnjdpn.github.io`. Un dépôt inaccessible
  constitue une action restante à nommer, jamais une synchronisation réussie.
- Exécuter les contrôles de la carte, puis relire au navigateur les routes et
  formats concernés, clavier et langues compris. Tester les formulaires sans
  transmettre de message réel. Les gardes déterministes ne jugent ni la fidélité
  des captures, ni les textes, ni la disponibilité distante.

Pour une release, ajouter la revue JSON décrite dans
[le contrat de revue](references/review.md) aux artefacts déjà utilisés par le
pipeline (ou dans un dossier temporaire pour une simulation). Aucun fichier
d'état perpétuel n'est nécessaire. Exécuter depuis la racine :

```sh
ruby .agents/skills/site-release-sync/scripts/check.rb --check
ruby .agents/skills/site-release-sync/scripts/check.rb --fingerprint
ruby .agents/skills/site-release-sync/scripts/check.rb --review "$SITE_REVIEW_FILE" --require-review
ruby .agents/skills/site-release-sync/scripts/check_test.rb
```

Les appels Fastlane utilisent `SITE_REVIEW_FILE`. Un résultat `pending` autorise
la préparation du candidat mais ne permet aucune nouvelle promesse publique.
Pour les sites `marketing/site.json`, `--check` exécute également
`scripts/marketing_site_test.rb` : la release et les CI partagent ainsi le même
contrôle de non-régression des snapshots publics, avec Minitest 6.0.6.
Une revue devenue obsolète après modification du candidat doit être refaite.
Après une publication explicitement autorisée, relire séparément l'URL servie,
ses téléchargements et chaque plateforme ; ne pas confondre CI et production.
