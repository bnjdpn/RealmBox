import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { AiCapability, BotPresence, ClientChoice, DialogueChattiness, GameDataInspection, LauncherProgress, LauncherStatus, RealmBackupSummary, RealmDiagnostics } from "./types";

declare global {
  interface Window { __TAURI_INTERNALS__?: unknown }
}

function browserStatus(): LauncherStatus {
  const base: LauncherStatus = {
    phase: "needsGameData",
    message: "Données de jeu requises",
    detail: "L’installation réelle est disponible dans l’application desktop RealmBox.",
    errorCode: null,
    progress: 0,
    installed: false,
    recoveryAvailable: false,
    botsEnabled: true,
    botCount: 50,
    requestedBotCount: 50,
    appliedBotCount: 50,
    botPresence: "natural",
    aiEnabled: false,
    aiModel: null,
    dialogueChattiness: "balanced",
    gameDataPath: null,
    accountName: null,
    accountPassword: null,
    clientChoice: "managedOpenWow",
    originalClientSupported: navigator.userAgent.includes("Windows"),
    platformLabel: "Aperçu navigateur",
    components: [
      { id: "client", label: "Client de jeu", state: "missing", detail: "À préparer" },
      { id: "database", label: "Sauvegarde du royaume", state: "missing", detail: "À préparer" },
      { id: "server", label: "Monde privé", state: "missing", detail: "À préparer" },
      { id: "bots", label: "Compagnons", state: "missing", detail: "Optionnels" },
      { id: "ai", label: "Dialogues vivants", state: "stopped", detail: "Selon cette machine" },
    ],
  };

  if (!import.meta.env.DEV) return base;
  const previewState = new URLSearchParams(window.location.search).get("previewState");
  if (previewState === "checking") return { ...base, phase: "checking", message: "Vérification de l’installation", progress: 18, components: [] };
  if (previewState === "installing") return {
    ...base,
    phase: "installing",
    message: "Préparation du serveur précompilé",
    progress: 63,
    components: base.components.map((component, index) => ({
      ...component,
      state: index < 2 ? "ready" : index === 2 ? "running" : component.state,
      detail: index < 2 ? "Installé" : index === 2 ? "En cours" : component.detail,
    })),
  };
  if (["ready", "running", "error"].includes(previewState ?? "")) {
    const running = previewState === "running";
    return {
      ...base,
      phase: previewState as "ready" | "running" | "error",
      message: running ? "Le monde est lancé" : "Installation terminée",
      detail: previewState === "error" ? "Le serveur local n’a pas répondu dans le délai prévu" : null,
      errorCode: previewState === "error" ? "worldServerTimeout" : null,
      progress: previewState === "error" ? 76 : 100,
      installed: true,
      gameDataPath: "/Jeux/RealmBox",
      accountName: "REALMBOX",
      accountPassword: "REALMBOX",
      components: base.components.map((component) => ({ ...component, state: running ? "running" : "ready", detail: running ? "Actif" : "Installé" })),
    };
  }
  return base;
}

export async function bootstrapLauncher(): Promise<LauncherStatus> {
  if (!window.__TAURI_INTERNALS__) return browserStatus();
  return invoke<LauncherStatus>("bootstrap_launcher");
}

export async function chooseGameData(): Promise<string | null> {
  if (!window.__TAURI_INTERNALS__) return null;
  const selected = await open({ directory: true, multiple: false, title: "Choisir le dossier WoW 3.3.5a" });
  return typeof selected === "string" ? selected : null;
}

export async function inspectAiCapability(): Promise<AiCapability> {
  if (!window.__TAURI_INTERNALS__) {
    const visualPreview = import.meta.env.DEV && ["ready", "running", "error"].includes(new URLSearchParams(window.location.search).get("previewState") ?? "");
    const previewEnglish = window.localStorage.getItem("realmbox-language") === "en";
    return visualPreview ? {
      state: "recommended",
      deviceName: previewEnglish ? "Visual preview" : "Aperçu visuel",
      ramGb: 16,
      modelId: "preview-model",
      modelName: previewEnglish ? "Recommended local model" : "Modèle local recommandé",
      ollamaModel: "preview:3b",
      grade: "A",
      estimatedTokensPerSecond: 42,
      downloadSizeGb: 2,
      diskAvailableGb: 80,
      diskSpaceSufficient: true,
      modelLicense: previewEnglish ? "Model license shown before installation" : "Licence du modèle affichée avant installation",
      detail: previewEnglish ? "Local visual example without download or execution." : "Exemple visuel local, sans téléchargement ni exécution.",
      sourceUrl: "https://www.canirun.ai/",
    } : {
      state: "unavailable",
      deviceName: null,
      ramGb: null,
      modelId: null,
      modelName: null,
      ollamaModel: null,
      grade: null,
      estimatedTokensPerSecond: null,
      downloadSizeGb: null,
      diskAvailableGb: null,
      diskSpaceSufficient: null,
      modelLicense: null,
      detail: "Le conseil matériel est disponible dans l’application desktop.",
      sourceUrl: "https://www.canirun.ai/",
    };
  }
  return invoke<AiCapability>("inspect_ai_capability");
}

export async function configureLocalDialogue(
  enabled: boolean,
  model: string | null,
): Promise<LauncherStatus> {
  if (!window.__TAURI_INTERNALS__) {
    return { ...browserStatus(), phase: "ready", installed: true, aiEnabled: enabled, aiModel: enabled ? model : null };
  }
  return invoke<LauncherStatus>("configure_local_dialogue", { enabled, model });
}

export async function configureDialogueChattiness(chattiness: DialogueChattiness): Promise<LauncherStatus> {
  if (!window.__TAURI_INTERNALS__) {
    return { ...browserStatus(), phase: "ready", installed: true, dialogueChattiness: chattiness };
  }
  return invoke<LauncherStatus>("configure_dialogue_chattiness", { chattiness });
}

export async function inspectGameData(gameDataPath: string): Promise<GameDataInspection> {
  if (!window.__TAURI_INTERNALS__) {
    return { path: gameDataPath, locale: "frFR", detail: "Aperçu navigateur sans lecture du disque." };
  }
  return invoke<GameDataInspection>("inspect_game_data", { gameDataPath });
}

export async function changeGameDataPath(gameDataPath: string): Promise<LauncherStatus> {
  if (!window.__TAURI_INTERNALS__) {
    return { ...browserStatus(), phase: "ready", installed: true, gameDataPath };
  }
  return invoke<LauncherStatus>("change_game_data_path", { gameDataPath });
}

export async function installRealm(
  gameDataPath: string,
  clientChoice: ClientChoice,
  botsEnabled: boolean,
  botCount: number,
  botPresence: BotPresence,
  aiEnabled: boolean,
  aiModel: string | null,
): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("install_realm", { request: { gameDataPath, clientChoice, botsEnabled, botCount, botPresence, aiEnabled, aiModel } });
}

export async function startRealm(botsEnabled: boolean, botCount: number, botPresence: BotPresence, aiEnabled: boolean): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("start_realm", { botsEnabled, botCount, botPresence, aiEnabled });
}

export async function stopRealm(): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("stop_realm");
}

export async function restoreLastRecovery(): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("restore_last_recovery");
}

export async function updatePlayerbotPopulation(botsEnabled: boolean, botCount: number, botPresence: BotPresence): Promise<LauncherStatus> {
  if (!window.__TAURI_INTERNALS__) {
    return { ...browserStatus(), phase: "running", installed: true, botsEnabled, botCount, requestedBotCount: botCount, appliedBotCount: botCount, botPresence };
  }
  return invoke<LauncherStatus>("update_playerbot_population", { botsEnabled, botCount, botPresence });
}

export async function inspectRealmBackup(): Promise<RealmBackupSummary | null> {
  if (!window.__TAURI_INTERNALS__) return null;
  return invoke<RealmBackupSummary | null>("inspect_realm_backup");
}

export async function createRealmBackup(): Promise<RealmBackupSummary> {
  if (!window.__TAURI_INTERNALS__) {
    return { createdAtUnixMs: Date.now(), sizeBytes: 4_194_304 };
  }
  return invoke<RealmBackupSummary>("create_realm_backup");
}

export async function getRealmDiagnostics(): Promise<RealmDiagnostics> {
  if (!window.__TAURI_INTERNALS__) {
    const english = document.documentElement.lang === "en";
    return {
      summary: english ? "No real diagnostics are available in browser preview." : "Aucun diagnostic réel dans l’aperçu navigateur.",
      component: "launcher",
      logsPath: english ? "Unavailable in browser preview" : "Indisponible dans l’aperçu navigateur",
      recentEntries: [],
    };
  }
  return invoke<RealmDiagnostics>("get_realm_diagnostics");
}

export async function subscribeLauncherProgress(
  onProgress: (progress: LauncherProgress) => void,
): Promise<UnlistenFn> {
  if (!window.__TAURI_INTERNALS__) return () => undefined;
  return listen<LauncherProgress>("realmbox://progress", (event) => onProgress(event.payload));
}

export async function subscribeLauncherStatus(
  onStatus: (status: LauncherStatus) => void,
): Promise<UnlistenFn> {
  if (!window.__TAURI_INTERNALS__) return () => undefined;
  return listen<LauncherStatus>("realmbox://status", (event) => onStatus(event.payload));
}
