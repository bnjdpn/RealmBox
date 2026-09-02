import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { AiCapability, ClientChoice, GameDataInspection, LauncherProgress, LauncherStatus } from "./types";

declare global {
  interface Window { __TAURI_INTERNALS__?: unknown }
}

function browserStatus(): LauncherStatus {
  return {
    phase: "needsGameData",
    message: "Données de jeu requises",
    detail: "L’installation réelle est disponible dans l’application desktop RealmBox.",
    progress: 0,
    installed: false,
    botsEnabled: true,
    aiEnabled: false,
    aiModel: null,
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
  if (!window.__TAURI_INTERNALS__) return {
    state: "unavailable",
    deviceName: null,
    ramGb: null,
    modelId: null,
    modelName: null,
    ollamaModel: null,
    grade: null,
    estimatedTokensPerSecond: null,
    detail: "Le conseil matériel est disponible dans l’application desktop.",
    sourceUrl: "https://www.canirun.ai/",
  };
  return invoke<AiCapability>("inspect_ai_capability");
}

export async function inspectGameData(gameDataPath: string): Promise<GameDataInspection> {
  if (!window.__TAURI_INTERNALS__) {
    return { path: gameDataPath, locale: "frFR", detail: "Aperçu navigateur sans lecture du disque." };
  }
  return invoke<GameDataInspection>("inspect_game_data", { gameDataPath });
}

export async function installRealm(
  gameDataPath: string,
  clientChoice: ClientChoice,
  botsEnabled: boolean,
  aiEnabled: boolean,
  aiModel: string | null,
): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("install_realm", { gameDataPath, clientChoice, botsEnabled, aiEnabled, aiModel });
}

export async function startRealm(botsEnabled: boolean, aiEnabled: boolean): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("start_realm", { botsEnabled, aiEnabled });
}

export async function stopRealm(): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("stop_realm");
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
