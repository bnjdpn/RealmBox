import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { LauncherProgress, LauncherStatus } from "./types";

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
    gameDataPath: null,
    accountName: null,
    accountPassword: null,
    components: [
      { id: "client", label: "Client de jeu", state: "missing", detail: "À préparer" },
      { id: "database", label: "Sauvegarde du royaume", state: "missing", detail: "À préparer" },
      { id: "server", label: "Monde privé", state: "missing", detail: "À préparer" },
      { id: "bots", label: "Compagnons", state: "missing", detail: "Optionnels" },
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

export async function installRealm(gameDataPath: string, botsEnabled: boolean): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("install_realm", { gameDataPath, botsEnabled });
}

export async function startRealm(botsEnabled: boolean): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("start_realm", { botsEnabled });
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
