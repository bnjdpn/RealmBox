# ADR 0002 — Copie de travail gérée par défaut

Statut : accepté pour implémentation.

RealmBox privilégiera une copie de travail dans son espace applicatif afin de ne pas modifier la source, stabiliser les chemins et permettre réparation/reprise. Le référencement direct restera avancé. L'implémentation devra mesurer l'espace, utiliser un staging atomique et gérer les volumes externes.

