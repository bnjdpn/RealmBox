# Contribuer à RealmBox / Contributing to RealmBox

Merci de contribuer. Les discussions et pull requests peuvent être rédigées en français ou en anglais.

Thank you for contributing. Issues and pull requests are welcome in French or English.

## Règles essentielles / Essential rules

- Ne joignez jamais de données WoW, MPQ, données extraites, bases utilisateur, secrets ou modèles. Never attach WoW data, MPQs, extracted data, user databases, secrets, or models.
- Gardez les effets plateforme derrière des interfaces typées et testez-les avec des fakes. Keep platform effects behind typed interfaces and test them with fakes.
- Épinglez tout upstream de production à un commit ou digest immuable. Pin every production upstream to an immutable commit or digest.
- Distinguez les preuves fake, automatisées, de build, manuelles et du parcours réel dans `STATUS.md`.

## Validation

```sh
pnpm install
pnpm verify
actionlint .github/workflows/*.yml
```

Une modification de sécurité, configuration, transition d’état ou récupération doit inclure un test de régression. A security, configuration, state-transition, or recovery change must include a regression test.

## Pull requests

Expliquez le comportement joueur, les plateformes touchées, les preuves exécutées et les limites restantes. Do not claim a full playable path from a unit test or a successful package build.
