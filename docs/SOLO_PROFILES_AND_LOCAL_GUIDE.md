# Profils solo et guide local

Ces deux fonctions restent dans le parcours joueur : aucun fichier serveur, mot de passe ou nom de volume n’est exposé dans l’interface. Elles sont implémentées dans les sources après RealmBox 0.4.0 et ne constituent pas encore une fonction distribuée ou qualifiée dans OpenWoW.

## Profils solo

Les profils modifient uniquement onze clés AzerothCore connues. Les autres lignes, commentaires et fins de ligne de `worldserver.conf` sont conservés.

| Profil | XP combat, quête, donjon, exploration et familier | Réputation | Argent | Métiers principaux | Quêtes normales en raid | Niveau et groupe raid exigés par les instances |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| **Normal** | ×1 | ×1 | ×1 | 2 | règle standard | oui |
| **Confort** | ×2 | ×2 | ×1 | 11 | autorisées | non |
| **Accéléré** | ×3 | ×3 | ×2 | 11 | autorisées | non |

La sélection montre toutes les valeurs avant application. Une modification est refusée tant que le monde ou le client géré tourne. RealmBox prend alors un instantané versionné et contrôlé par SHA-256, hors du runtime remplaçable, puis publie la nouvelle configuration atomiquement. Les instantanés ne contiennent que ces onze valeurs : ni mot de passe, ni base, ni données de personnage.

Un journal durable rend l’opération reprenable après interruption. Il est écrit, synchronisé et relu avant publication sous son nom final non écrasable ; un fichier final incomplet est une corruption et bloque l’opération. Le prochain changement ou démarrage du monde reprend le journal avant d’agir, depuis les valeurs typées de l’instantané validé plutôt que depuis le catalogue produit courant. Une seule instance de RealmBox peut piloter ce runtime à la fois.

**Revenir aux règles précédentes** restaure les onze valeurs sauvegardées sans toucher aux options non gérées. Ce retour ne retire jamais l’expérience, l’argent ou les métiers déjà acquis par un personnage et ne modifie pas la difficulté des ennemis. Il ne remplace donc ni AutoBalance ni une migration de personnage.

## Guide local

Le guide recherche par nom jusqu’à huit quêtes ou objets dans le catalogue `acore_world` déjà présent sur la machine. Le joueur choisit explicitement le type et saisit entre 2 et 64 caractères. Les titres et descriptions utilisent `frFR` ou `enUS`, avec repli sur le texte de base du monde.

Ce n’est ni un assistant IA, ni un guide de progression :

- aucune requête n’est envoyée à Ollama, au Web ou à un service tiers ;
- la recherche ne lit ni le personnage, ni l’inventaire, ni le journal de quêtes ;
- la réponse expose sa provenance, sa date d’observation et un état complet, partiel, vide ou indisponible ;
- les champs, le flux SQL et la réponse sont bornés ; les erreurs privées sont remplacées par un état générique.

Le lanceur refuse de démarrer une base dont le volume joueur a disparu. Si la base existe mais est arrêtée, il démarre uniquement son service, sans build, téléchargement ni dépendance, puis tente son arrêt, y compris après une erreur ou un timeout du démarrage ou de la recherche. Une base déjà active reste active. Les deux requêtes SQL sont constantes, limitées à huit lignes, exécutées dans `START TRANSACTION READ ONLY`, assorties du délai MySQL `MAX_EXECUTION_TIME(2000)` et ne comportent aucune instruction de mutation. Le terme utilisateur est transmis uniquement sous forme hexadécimale UTF-8 validée.

Le délai SQL ne borne pas Docker : chaque processus hôte est donc aussi surveillé, avec 10 secondes pour `volume inspect`, `compose ps` et la recherche, 125 secondes pour `compose up`, puis 15 secondes pour `compose stop`. À l’échéance, seul le groupe de processus ou Job Object possédé est terminé, sans thread d’attente détaché. Si le daemon reste inaccessible, l’arrêt du conteneur n’est pas garanti et l’erreur indique qu’il reste à vérifier. La capture utilise des fichiers temporaires privés supprimés en fin d’appel, avec une lecture maximale de 64 KiB ; un descendant héritant de la sortie ne peut pas bloquer sa lecture.

RealmBox réutilise la connexion root locale déjà nécessaire au runtime, parce que créer un nouveau compte SQL serait une mutation de schéma exigeant sa propre migration protégée. La sûreté repose donc ici sur la requête fermée, la transaction en lecture seule, l’absence de port public et les bornes ; un compte `SELECT` dédié reste une amélioration différée à introduire derrière une sauvegarde pré-migration.

## Preuves et limites

- Le moteur de profils, les instantanés, la reprise et les gardes de runtime sont testés avec fichiers temporaires et runners factices.
- Le script `pnpm test:guide-sql` extrait les deux requêtes exactes de Rust et les exécute dans un MySQL 8.4.11 isolé, sans réseau, volume, port ni donnée de jeu. Il vérifie les recherches FR/EN, le cadrage cinq colonnes, les bornes et le refus réel d’un `INSERT` dans la transaction en lecture seule.
- Les tests UI couvrent aperçu, application, retour, reprise incertaine, états vide/partiel/indisponible, français/anglais et axe.

Ces preuves n’établissent pas le comportement d’un bundle distribué, la compatibilité avec un monde joueur existant ou le rendu dans OpenWoW. Aucun profil n’a été appliqué et aucune requête n’a été faite sur le royaume réel pendant ce lot.
