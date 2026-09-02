# Dépannage

- Aperçu visuel seul : `pnpm dev:preview`. Il indique que l'installation réelle nécessite l'application desktop.
- « Docker Desktop doit être installé et démarré » : ouvrir Docker Desktop, attendre que le moteur réponde, puis revérifier.
- « le dossier choisi ne contient pas Data » : sélectionner le dossier parent de `Data`, ou `Data` lui-même.
- Échec OpenWoW : consulter `logs/openwow-download.log`; RealmBox refuse un fichier dont le SHA-256 diffère.
- Échec de build/extraction/import : consulter les logs gérés correspondants. Une installation interrompue n'écrit pas le manifeste final.
- Lancement suivant en erreur : vérifier que le dossier de jeu n'a pas été déplacé et que Docker Desktop tourne.
- Les données sont absentes : ne pas contourner la validation et ne télécharger aucun contenu propriétaire via RealmBox.
