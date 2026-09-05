# Revue liée au candidat

Un fichier JSON court appartient au dossier d'artefacts de la release existante,
hors des sources publiées. Il peut aussi être temporaire pour un contrôle local.
Le garde n'appelle aucun réseau et ne publie rien. Il vérifie la complétude et
les contradictions de la décision saisie ; il ne certifie pas sa véracité.

```json
{
  "source_commit": "SHA Git complet du candidat",
  "source_fingerprint": "empreinte SHA-256 des sources courantes",
  "version": "version du candidat",
  "impact": "none",
  "reason": "Extraction interne du calcul ; UI, résultats, offre et données inchangés.",
  "site": "unchanged",
  "screenshots": "unchanged",
  "catalogue": "unchanged",
  "languages_reviewed": ["fr-FR", "en-US"],
  "platforms": [{"platform": "iOS", "availability": "unchanged", "claims_changed": false}]
}
```

- Obtenir `source_commit` et `source_fingerprint` avec
  `ruby .agents/skills/site-release-sync/scripts/check.rb --fingerprint`, après
  la préparation du candidat. L'empreinte lie le commit et le contenu actuel
  des fichiers suivis modifiés/supprimés ainsi que les nouveaux fichiers non
  ignorés. Une édition non committée du site après la revue la rend invalide.
  Le JSON de revue doit être hors du checkout ou dans ses artefacts ignorés,
  pour éviter l'autoréférence. Les sorties ignorées restent vérifiées par les
  contrats médias/binaire existants ; ce hash ne remplace pas leur provenance.
- `source_commit` est le SHA de `git rev-parse HEAD`. `version` identifie le
  candidat, ou `site` pour une modification web seule. Le garde vérifie le SHA
  courant et l'empreinte ; la revue doit couvrir les changements non committés. Elle ne
  remplace pas le manifeste exact du binaire ni la provenance des captures.
- `impact` : `none`, `ui`, `content`. `reason` explique concrètement ce qui a été
  vérifié et, si une surface est inchangée, pourquoi. `site`, `screenshots` et
  `catalogue` : `unchanged`, `updated`, `deferred`. Une capture d'UI devenue
  inexacte doit être `updated` ou `deferred`, jamais `unchanged`.
- `deferred` signifie préparé dans une branche/patch hors publication et nécessite
  `pending_work` décrivant l'emplacement et le travail à finir. Un site associé
  inaccessible utilise aussi cette valeur. Ce n'est pas une revue terminée.
- `languages_reviewed` liste les locales affectées contrôlées. Pour `none`, la
  liste peut être vide : aucune traduction modifiée ne demande un test artificiel.
- Chaque plateforme concernée a son propre `availability` : `unchanged`,
  `pending`, `released`, et `claims_changed` booléen. Une release iOS ne prouve
  pas une disponibilité Android, Windows ou macOS.
- Une promesse nouvelle (`claims_changed: true`) exige `released` ainsi que
  `public_version`, `public_url` (HTTPS), `checked_at` (ISO 8601) et `evidence`
  décrivant la lecture publique et les offres/limitations vérifiées. La date doit
  être récente (7 jours maximum). Un tag, un merge et une soumission ne sont
  jamais cette preuve. Le lecteur vérifie ces faits à la source ; le garde ne
  fait qu'en vérifier la présence et la cohérence.

Pour une UI en attente : `impact: ui`, `site/screenshots/catalogue: deferred`,
`pending_work` localisant les captures et textes préparés, plateforme `pending`,
`claims_changed: false`. Les pages actuellement publiables continuent à décrire
la version déjà distribuée. Pour une release rendue publique, refaire la revue
avec les lectures exactes de chaque plateforme, puis seulement proposer la
publication des nouveautés si elle n'est pas encore autorisée.

`SITE_REVIEW_FILE` est lu par Fastlane ; il n'accorde aucune autorisation de
soumission ou publication. Les CI sans ce fichier contrôlent l'intégrité du
workflow et les scénarios simulés, et annoncent explicitement la revue de contenu
restant humaine. Ne pas fabriquer une revue pour faire passer la CI.
