import { invoke } from "@tauri-apps/api/core";
import { fakeDashboard, type Dashboard } from "./types";

declare global {
  interface Window { __TAURI_INTERNALS__?: unknown }
}

const wait = (milliseconds: number) => new Promise((resolve) => window.setTimeout(resolve, milliseconds));

async function callOrFake<T>(command: string, fallback: () => Promise<T>): Promise<T> {
  if (window.__TAURI_INTERNALS__) return invoke<T>(command);
  return fallback();
}

export async function prepareWorld(onProgress: (progress: number, message: string) => void): Promise<Dashboard> {
  return callOrFake("prepare_fake_world", async () => {
    const stages = [
      [12, "Lecture des données"],
      [31, "Préparation du monde"],
      [53, "Création des habitants"],
      [72, "Préparation de vos compagnons"],
      [88, "Installation de l’IA locale"],
      [100, "Vérification finale"],
    ] as const;
    for (const [progress, message] of stages) {
      onProgress(progress, message);
      await wait(40);
    }
    return structuredClone(fakeDashboard);
  });
}

export async function startWorld(onProgress: (progress: number, message: string) => void): Promise<Dashboard> {
  return callOrFake("start_fake_world", async () => {
    for (const [progress, message] of [[20, "Réveil du monde"], [48, "Préparation de vos compagnons"], [72, "Démarrage de l’IA locale"], [100, "Ouverture du jeu"]] as const) {
      onProgress(progress, message);
      await wait(45);
    }
    return { ...structuredClone(fakeDashboard), sessionRunning: true };
  });
}

export async function stopWorld(): Promise<Dashboard> {
  return callOrFake("stop_fake_world", async () => ({ ...structuredClone(fakeDashboard), sessionRunning: false }));
}

export async function talkToCompanion(companionId: string, message: string): Promise<string> {
  return callOrFake("talk_to_fake_companion", async () => {
    if (!message.trim()) return "";
    if (companionId === "melya") return "Melya : Je suis prête. Gardons juste un peu de mana avant le prochain combat.";
    return "Thoran : Je passe devant. Restons groupés.";
  });
}
