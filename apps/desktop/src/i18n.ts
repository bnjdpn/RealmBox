export type Language = "fr" | "en";

const fr = {
  localOnly: "LOCAL UNIQUEMENT", world: "Mon monde", companions: "Compagnons", dialogues: "Dialogues", diagnostics: "Diagnostic", language: "Langue",
  settings: "Réglages", close: "Fermer", back: "Retour", installationOptions: "Options d’installation", changeFolder: "Changer de dossier", openDiagnostics: "Voir le diagnostic", dataReady: "Données de jeu reconnues",
  installTitle: "Préparer mon monde", installBody: "Choisissez vos données de jeu. RealmBox installe ensuite les composants ouverts et garde tout sur cet ordinateur.",
  gameClient: "Client de jeu", managedClient: "OpenWoW géré par RealmBox", managedClientHelp: "Recommandé · téléchargé automatiquement et vérifié.",
  originalClient: "Mon client original", originalClientHelp: "Windows uniquement · RealmBox sauvegarde la configuration du royaume.", originalUnavailable: "Disponible uniquement sur Windows x64.",
  gameFolder: "Dossier du jeu", changeGameFolder: "Changer le dossier du client", gameFolderUpdated: "Nouveau dossier vérifié et enregistré.",
  managedPathHelp: "OpenWoW reste géré par RealmBox. Ce dossier fournit les données du jeu.", originalPathHelp: "RealmBox lancera Wow.exe depuis ce dossier.", stopToChangeGameFolder: "Arrêtez le monde avant de changer ce dossier.",
  gameData: "Données de jeu 3.3.5a", chooseData: "Choisir le dossier qui contient Data", browse: "Choisir",
  dataRequirement: "Copie compatible build 12340 requise. RealmBox ne télécharge aucune donnée propriétaire.",
  populate: "Peupler le monde", populateHelp: "Ajoute des aventuriers autonomes et permet de former une équipe en jeu.",
  population: "Population souhaitée", populationHelp: "RealmBox applique automatiquement une limite sûre selon la mémoire Docker.",
  ai: "Dialogues locaux", aiChecking: "Vérification de cette machine…", aiUnavailable: "Aucun petit modèle confortable détecté.",
  aiPrivacy: "L’analyse envoie uniquement le processeur, le nombre de cœurs et la mémoire à CanIRun.",
  dialoguesTitle: "Dialogues locaux", dialoguesBody: "RealmBox choisit automatiquement un petit modèle rapide adapté à cette machine.",
  canirunDecision: "Décision RealmBox avec CanIRun", recommendedModel: "Modèle retenu", downloadSize: "Téléchargement", estimatedSpeed: "Vitesse estimée", modelLicense: "Licence du modèle", hardware: "Machine évaluée",
  activateDialogues: "Télécharger et activer", deactivateDialogues: "Désactiver", dialogueReady: "Les dialogues seront actifs au prochain lancement du monde.",
  dialogueLocalProof: "Après le téléchargement, Ollama écoute uniquement sur 127.0.0.1 et le jeu fonctionne sans réseau.", closeToChange: "Le monde doit être arrêté avant de préparer ou modifier les dialogues.", stopForDialogues: "Arrêter le monde pour continuer",
  dialogueDownloadConsent: "Le téléchargement ne démarre qu’après cette confirmation.", dialogueFailed: "L’activation n’a pas abouti.", modelInstalled: "Installé localement",
  account: "Compte local", accountHelp: "À saisir dans le client. Accessible uniquement sur ce serveur local.",
  install: "Installer", play: "Jouer", inGame: "En jeu", stop: "Arrêter le monde", wait: "Patientez…", checkAgain: "Réessayer", components: "Composants du royaume",
  runningTitle: "Votre monde est ouvert", runningBody: "Le serveur local et le client sont en cours d’exécution.",
  readyTitle: "Votre monde est prêt", readyBody: "Tous les composants requis sont installés et vérifiés.",
  checkingTitle: "Vérification en cours", checkingBody: "RealmBox contrôle l’installation locale.",
  startingTitle: "Ouverture du monde", stoppingTitle: "Fermeture du monde", installingTitle: "Installation en cours",
  errorTitle: "Une étape n’a pas abouti", genericError: "Une étape locale n’a pas abouti.", genericRecovery: "Ouvrez Diagnostic pour voir le composant concerné, puis réessayez.",
  dockerError: "Docker Desktop n’est pas prêt.", dockerRecovery: "Ouvrez Docker Desktop, attendez qu’il soit démarré, puis réessayez.",
  dataError: "Les données de jeu choisies sont incomplètes.", dataRecovery: "Choisissez une copie 3.3.5a build 12340 complète.",
  downloadError: "Un composant n’a pas pu être téléchargé.", downloadRecovery: "Vérifiez la connexion Internet, puis réessayez.",
  clientError: "Le client de jeu n’a pas pu démarrer.", clientRecovery: "Fermez toute alerte du client, puis relancez le monde.",
  serverError: "Le serveur local n’est pas prêt.", serverRecovery: "Vérifiez Docker Desktop et consultez Diagnostic avant de réessayer.",
  cause: "Cause", recovery: "À faire", progress: "Progression", selected: "Choisi", active: "Actif", ready: "Prêt", off: "Désactivé",
  companionsTitle: "Population du monde", companionsBodyReady: "Choisissez la population utilisée au prochain démarrage.",
  companionsBodyRunning: "Appliquez la nouvelle population sans fermer le client. La connexion des bots peut prendre quelques instants.",
  requestedPopulation: "Population appliquée", team: "Équipe en jeu", teamHelp: "Utilisez l’addon RealmBox pour former et diriger une équipe équilibrée de quatre compagnons.",
  applyNow: "Appliquer maintenant", applied: "Population appliquée sans redémarrer le client.", startToApply: "Lancez le monde pour appliquer une modification à chaud.",
  diagnosticsTitle: "Diagnostic local", diagnosticsBody: "Seuls les avertissements et erreurs des journaux gérés sont affichés. Les lignes sensibles sont masquées.",
  refresh: "Actualiser", copy: "Copier le diagnostic", copied: "Diagnostic copié", affectedComponent: "Composant concerné", logsFolder: "Dossier des journaux",
  noRecentErrors: "Aucun avertissement ou erreur récent.", noDiagnostic: "Le diagnostic sera disponible après la vérification locale.",
  componentClient: "Client de jeu", componentDatabase: "Sauvegarde locale", componentServer: "Serveur local", componentBots: "Compagnons", componentAi: "Dialogues locaux", componentLauncher: "RealmBox",
  discreet: "Discret", light: "Léger", balanced: "Équilibré", dense: "Dense", veryDense: "Très dense", versionSuffix: "serveur local uniquement",
} as const;

export type Copy = Record<keyof typeof fr, string>;

const en: Copy = {
  localOnly: "LOCAL ONLY", world: "My world", companions: "Companions", dialogues: "Dialogue", diagnostics: "Diagnostics", language: "Language",
  settings: "Settings", close: "Close", back: "Back", installationOptions: "Installation options", changeFolder: "Choose another folder", openDiagnostics: "View diagnostics", dataReady: "Game data recognized",
  installTitle: "Set up my world", installBody: "Choose your game data. RealmBox then installs the open components and keeps everything on this computer.",
  gameClient: "Game client", managedClient: "OpenWoW managed by RealmBox", managedClientHelp: "Recommended · downloaded automatically and verified.",
  originalClient: "My original client", originalClientHelp: "Windows only · RealmBox backs up the realm configuration.", originalUnavailable: "Available on Windows x64 only.",
  gameFolder: "Game folder", changeGameFolder: "Change client folder", gameFolderUpdated: "New folder verified and saved.",
  managedPathHelp: "OpenWoW remains managed by RealmBox. This folder provides the game data.", originalPathHelp: "RealmBox will launch Wow.exe from this folder.", stopToChangeGameFolder: "Stop the world before changing this folder.",
  gameData: "3.3.5a game data", chooseData: "Choose the folder containing Data", browse: "Choose",
  dataRequirement: "A compatible build 12340 copy is required. RealmBox does not download proprietary game data.",
  populate: "Populate the world", populateHelp: "Adds autonomous adventurers and lets you form a party in game.",
  population: "Desired population", populationHelp: "RealmBox automatically applies a safe limit based on Docker memory.",
  ai: "Local dialogue", aiChecking: "Checking this computer…", aiUnavailable: "No comfortable small model was detected.",
  aiPrivacy: "The check only sends the processor, core count and memory to CanIRun.",
  dialoguesTitle: "Local dialogue", dialoguesBody: "RealmBox automatically chooses a small, fast model suited to this computer.",
  canirunDecision: "RealmBox decision with CanIRun", recommendedModel: "Selected model", downloadSize: "Download", estimatedSpeed: "Estimated speed", modelLicense: "Model license", hardware: "Evaluated computer",
  activateDialogues: "Download and enable", deactivateDialogues: "Disable", dialogueReady: "Dialogue will be active the next time the world starts.",
  dialogueLocalProof: "After download, Ollama only listens on 127.0.0.1 and the game works without a network.", closeToChange: "The world must be stopped before local dialogue can be prepared or changed.", stopForDialogues: "Stop the world to continue",
  dialogueDownloadConsent: "The download only starts after this confirmation.", dialogueFailed: "Dialogue setup did not complete.", modelInstalled: "Installed locally",
  account: "Local account", accountHelp: "Enter this in the client. It is only accessible on this local server.",
  install: "Install", play: "Play", inGame: "In game", stop: "Stop the world", wait: "Please wait…", checkAgain: "Try again", components: "Realm components",
  runningTitle: "Your world is open", runningBody: "The local server and game client are running.", readyTitle: "Your world is ready", readyBody: "All required components are installed and verified.",
  checkingTitle: "Checking your installation", checkingBody: "RealmBox is checking the local installation.", startingTitle: "Opening your world", stoppingTitle: "Closing your world", installingTitle: "Installing your world",
  errorTitle: "Something needs attention", genericError: "A local step did not complete.", genericRecovery: "Open Diagnostics to see the affected component, then try again.",
  dockerError: "Docker Desktop is not ready.", dockerRecovery: "Open Docker Desktop, wait for it to start, then try again.", dataError: "The selected game data is incomplete.", dataRecovery: "Choose a complete 3.3.5a build 12340 copy.",
  downloadError: "A component could not be downloaded.", downloadRecovery: "Check your internet connection, then try again.", clientError: "The game client could not start.", clientRecovery: "Close any client alert, then open the world again.",
  serverError: "The local server is not ready.", serverRecovery: "Check Docker Desktop and review Diagnostics before trying again.",
  cause: "Cause", recovery: "What to do", progress: "Progress", selected: "Selected", active: "Running", ready: "Ready", off: "Off",
  companionsTitle: "World population", companionsBodyReady: "Choose the population to use the next time the world starts.", companionsBodyRunning: "Apply a new population without closing the client. Bot connections may take a moment.",
  requestedPopulation: "Applied population", team: "In-game party", teamHelp: "Use the RealmBox addon to form and direct a balanced party of four companions.", applyNow: "Apply now", applied: "Population applied without restarting the client.", startToApply: "Open the world to apply a live change.",
  diagnosticsTitle: "Local diagnostics", diagnosticsBody: "Only warnings and errors from managed logs are shown. Sensitive lines are redacted.", refresh: "Refresh", copy: "Copy diagnostics", copied: "Diagnostics copied", affectedComponent: "Affected component", logsFolder: "Logs folder",
  noRecentErrors: "No recent warning or error.", noDiagnostic: "Diagnostics will be available after the local check.", componentClient: "Game client", componentDatabase: "Local save", componentServer: "Local server", componentBots: "Companions", componentAi: "Local dialogue", componentLauncher: "RealmBox",
  discreet: "Minimal", light: "Light", balanced: "Balanced", dense: "Dense", veryDense: "Very dense", versionSuffix: "local server only",
};

export const messages: Record<Language, Copy> = { fr, en };

export function preferredLanguage(): Language {
  const stored = localStorage.getItem("realmbox-language");
  if (stored === "fr" || stored === "en") return stored;
  return navigator.language.toLowerCase().startsWith("fr") ? "fr" : "en";
}
