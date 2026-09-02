# ADR 0003 — Frontières de runtime typées

Statut : accepté.

Toutes les opérations système passent par des traits Rust injectables. Le fake implémente les mêmes frontières. Cela permet de développer sur macOS et de tester les chemins Windows sans prétendre à une validation Windows réelle.

